use std::{fs, path::PathBuf};

use engine_asset::prelude::*;

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
}
fn texture_rgba_bytes(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend_from_slice(rgba);
    bytes
}

fn audio_bytes() -> Vec<u8> {
    b"NGA_AUDIO_V1\nsample_rate=44100\nchannels=1\nformat=f32\nsamples=0.0,0.5,-0.5\nstreaming=false\n"
        .to_vec()
}

fn animation_source_bytes() -> Vec<u8> {
    b"NGA_ANIMATION_SOURCE_V1\nduration=1.0\nticks_per_second=24.0\ntrack=node:Hero\ntranslation=0.0:0,0,0\nrotation=0.0:0,0,0,1\nscale=0.0:1,1,1\n".to_vec()
}

fn animation_runtime_bytes() -> Vec<u8> {
    b"NGA_ANIMATION_V1\nduration=1.0\nticks_per_second=24.0\ntrack=node:Hero\ntranslation=0.0:0,0,0\nrotation=0.0:0,0,0,1\nscale=0.0:1,1,1\n".to_vec()
}

fn skeleton_source_bytes() -> Vec<u8> {
    b"NGA_SKELETON_SOURCE_V1\nbone=Root;bind=1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1\nbone=Child;parent=0;bind=1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1\n".to_vec()
}

fn skeleton_runtime_bytes() -> Vec<u8> {
    b"NGA_SKELETON_V1\nbone=Root;bind=1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1\nbone=Child;parent=0;bind=1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1\n".to_vec()
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

fn wav_format_bytes(
    sample_rate: u32,
    channels: u16,
    audio_format: u16,
    bits_per_sample: u16,
    data: &[u8],
) -> Vec<u8> {
    let bytes_per_sample = u32::from(bits_per_sample / 8);
    let data_len = data.len() as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(4 + (8 + 16) + (8 + data_len)).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&audio_format.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * u32::from(channels) * bytes_per_sample).to_le_bytes());
    bytes.extend_from_slice(&((u32::from(channels) * bytes_per_sample) as u16).to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.extend_from_slice(data);
    bytes
}

fn wav_pcm_bytes(sample_rate: u32, channels: u16, bits_per_sample: u16, data: &[u8]) -> Vec<u8> {
    wav_format_bytes(sample_rate, channels, 1, bits_per_sample, data)
}

fn wav_ima_adpcm_bytes(
    sample_rate: u32,
    channels: u16,
    block_align: u16,
    samples_per_block: u16,
    data: &[u8],
) -> Vec<u8> {
    let data_len = data.len() as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(4 + (8 + 20) + (8 + data_len)).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&20u32.to_le_bytes());
    bytes.extend_from_slice(&17u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(
        &(sample_rate * u32::from(block_align) / u32::from(samples_per_block)).to_le_bytes(),
    );
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&samples_per_block.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.extend_from_slice(data);
    bytes
}

fn wav_ms_adpcm_bytes(
    sample_rate: u32,
    channels: u16,
    block_align: u16,
    samples_per_block: u16,
    data: &[u8],
) -> Vec<u8> {
    let coefficients = [(256i16, 0i16)];
    let extension_size = 4 + coefficients.len() as u16 * 4;
    let fmt_len = 18 + u32::from(extension_size);
    let data_len = data.len() as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(4 + (8 + fmt_len) + (8 + data_len)).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&fmt_len.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(
        &(sample_rate * u32::from(block_align) / u32::from(samples_per_block)).to_le_bytes(),
    );
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&extension_size.to_le_bytes());
    bytes.extend_from_slice(&samples_per_block.to_le_bytes());
    bytes.extend_from_slice(&(coefficients.len() as u16).to_le_bytes());
    for (coefficient_1, coefficient_2) in coefficients {
        bytes.extend_from_slice(&coefficient_1.to_le_bytes());
        bytes.extend_from_slice(&coefficient_2.to_le_bytes());
    }
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.extend_from_slice(data);
    bytes
}

fn wav_extensible_bytes(
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    subformat_tag: u16,
    data: &[u8],
) -> Vec<u8> {
    let bytes_per_sample = u32::from(bits_per_sample / 8);
    let block_align = (u32::from(channels) * bytes_per_sample) as u16;
    wav_extensible_bytes_with_format_field(
        sample_rate,
        channels,
        block_align,
        bits_per_sample,
        bits_per_sample,
        subformat_tag,
        data,
    )
}

fn wav_extensible_ima_adpcm_bytes(
    sample_rate: u32,
    channels: u16,
    block_align: u16,
    samples_per_block: u16,
    data: &[u8],
) -> Vec<u8> {
    wav_extensible_bytes_with_format_field(
        sample_rate,
        channels,
        block_align,
        4,
        samples_per_block,
        17,
        data,
    )
}

fn wav_extensible_bytes_with_format_field(
    sample_rate: u32,
    channels: u16,
    block_align: u16,
    bits_per_sample: u16,
    format_field: u16,
    subformat_tag: u16,
    data: &[u8],
) -> Vec<u8> {
    let data_len = data.len() as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(4 + (8 + 40) + (8 + data_len)).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&0xfffeu16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
    bytes.extend_from_slice(&22u16.to_le_bytes());
    bytes.extend_from_slice(&format_field.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&u32::from(subformat_tag).to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0x0010u16.to_le_bytes());
    bytes.extend_from_slice(&[0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.extend_from_slice(data);
    bytes
}

fn font_bytes() -> Vec<u8> {
    b"NGA_FONT_V1\nfamily=Debug Sans\nglyph=char=A;size=2x1;bitmap=0,255\n".to_vec()
}

fn ttf_font_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes
}

fn otf_font_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"OTTO");
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes
}

fn ogg_vorbis_audio_bytes(sample_rate: u32, channels: u16) -> Vec<u8> {
    let mut packet = Vec::new();
    packet.push(0x01);
    packet.extend_from_slice(b"vorbis");
    packet.extend_from_slice(&0u32.to_le_bytes());
    packet.push(u8::try_from(channels).unwrap_or(u8::MAX));
    packet.extend_from_slice(&sample_rate.to_le_bytes());
    packet.extend_from_slice(&0u32.to_le_bytes());
    packet.extend_from_slice(&0u32.to_le_bytes());
    packet.extend_from_slice(&0u32.to_le_bytes());
    make_ogg_page(packet)
}

fn ogg_opus_audio_bytes(sample_rate: u32, channels: u16) -> Vec<u8> {
    let mut packet = Vec::new();
    packet.extend_from_slice(b"OpusHead");
    packet.push(1);
    packet.push(u8::try_from(channels).unwrap_or(u8::MAX));
    packet.extend_from_slice(&0u16.to_le_bytes());
    packet.extend_from_slice(&sample_rate.to_le_bytes());
    packet.extend_from_slice(&0i16.to_le_bytes());
    packet.push(0);
    make_ogg_page(packet)
}

fn make_ogg_page(packet: Vec<u8>) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"OggS");
    bytes.push(0x00);
    bytes.push(0x00);
    bytes.extend_from_slice(&0u64.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.push(1u8);
    bytes.push(u8::try_from(packet.len()).expect("ogg test packet must be single-segment"));
    bytes.extend_from_slice(&packet);
    bytes
}

fn physics_mesh_bytes() -> Vec<u8> {
    b"NGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\n".to_vec()
}

fn metadata_line_with_fields(id: AssetId, path: &AssetPath, field_count: usize) -> String {
    let mut fields = vec![
        id.raw().to_string(),
        AssetTypeId::of::<Texture>().raw().to_string(),
        path.display_string(),
        path.display_string(),
        path.display_string(),
        "TextureImporter".to_owned(),
        "1".to_owned(),
        "11".to_owned(),
        "22".to_owned(),
        "33".to_owned(),
        "44".to_owned(),
        String::new(),
    ];
    if field_count >= 13 {
        fields.push("label".to_owned());
    }
    if field_count >= 14 {
        fields.push("quality=high".to_owned());
    }
    fields.join("|")
}

fn legacy_v0_metadata_line(
    id: AssetId,
    path: &AssetPath,
    source_path: &AssetPath,
    cooked_path: &AssetPath,
    dependencies: &[AssetId],
) -> String {
    [
        id.raw().to_string(),
        AssetTypeId::of::<Texture>().raw().to_string(),
        path.display_string(),
        source_path.display_string(),
        cooked_path.display_string(),
        "TextureImporter".to_owned(),
        "1".to_owned(),
        "11".to_owned(),
        "22".to_owned(),
        "33".to_owned(),
        dependencies
            .iter()
            .map(|id| id.raw().to_string())
            .collect::<Vec<_>>()
            .join(","),
    ]
    .join("|")
}

fn mesh_bytes() -> Vec<u8> {
    b"v 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\n".to_vec()
}

fn simple_binary_mesh_bytes() -> Vec<u8> {
    mesh_binary_bytes(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[0, 1, 2],
    )
}

fn simple_u16_index_binary_mesh_bytes() -> Vec<u8> {
    let mut bytes = b"NGA_MESH_BINARY_V1\n".to_vec();
    push_mesh_binary_u32(&mut bytes, 3);
    push_mesh_binary_u32(&mut bytes, 3);
    push_mesh_binary_u32(&mut bytes, 16);
    push_mesh_binary_u32(&mut bytes, 0);
    for vertex in [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        push_mesh_binary_f32s(&mut bytes, &vertex);
    }
    for index in [0u16, 1u16, 2u16] {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    bytes
}

fn unoptimized_mesh_binary_bytes() -> Vec<u8> {
    mesh_binary_bytes(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[0, 4, 3],
    )
}

fn binary_mesh_bytes() -> Vec<u8> {
    mesh_binary_bytes(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0.0, 0.0, 1.0]; 3],
        &[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        &[&[[0.25, 0.25], [0.75, 0.25], [0.25, 0.75]]],
        &[],
        &[
            [0u16, 1u16, 2u16, 3u16],
            [0u16, 0u16, 0u16, 0u16],
            [1u16, 2u16, 3u16, 4u16],
        ],
        &[
            [0.7, 0.2, 0.1, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.25, 0.25, 0.25, 0.25],
        ],
        &[0, 1, 2],
    )
}

fn converted_source_binary_mesh_bytes() -> Vec<u8> {
    mesh_binary_bytes(
        &[[0.0, 0.0, 0.0], [1.5, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0.0, 0.0, 1.0]; 3],
        &[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        &[&[[0.25, 0.25], [0.75, 0.25], [0.25, 0.75]]],
        &[[1.0, 0.0, 0.0, 1.0]; 3],
        &[
            [0u16, 1u16, 2u16, 3u16],
            [0u16, 0u16, 0u16, 0u16],
            [1u16, 2u16, 3u16, 4u16],
        ],
        &[
            [0.7, 0.2, 0.1, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.25, 0.25, 0.25, 0.25],
        ],
        &[0, 1, 2],
    )
}

fn skinned_binary_mesh_bytes() -> Vec<u8> {
    mesh_binary_bytes(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[],
        &[],
        &[],
        &[],
        &[
            [0u16, 1u16, 0u16, 0u16],
            [1u16, 0u16, 0u16, 0u16],
            [0u16, 0u16, 0u16, 0u16],
        ],
        &[
            [0.75, 0.25, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        ],
        &[0, 1, 2],
    )
}

fn obj_binary_mesh_bytes() -> Vec<u8> {
    mesh_binary_bytes(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        &[[0.0, 0.0, 1.0]; 4],
        &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        &[],
        &[[1.0, 0.0, 0.0, 1.0]; 4],
        &[],
        &[],
        &[0, 1, 2, 0, 2, 3],
    )
}

fn mesh_binary_bytes(
    vertices: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    uv_sets: &[&[[f32; 2]]],
    tangents: &[[f32; 4]],
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    indices: &[u32],
) -> Vec<u8> {
    let mut flags = 0;
    if !normals.is_empty() {
        flags |= 1;
    }
    if !uvs.is_empty() {
        flags |= 2;
    }
    if !tangents.is_empty() {
        flags |= 4;
    }
    if !joints.is_empty() {
        flags |= 8;
    }

    let mut secondary_uv_mask = 0u32;
    for (index, uv_set) in uv_sets.iter().enumerate() {
        if !uv_set.is_empty() {
            secondary_uv_mask |= 1 << index;
        }
    }

    let mut bytes = b"NGA_MESH_BINARY_V1\n".to_vec();
    push_mesh_binary_u32(&mut bytes, vertices.len() as u32);
    push_mesh_binary_u32(&mut bytes, indices.len() as u32);
    push_mesh_binary_u32(&mut bytes, flags);
    push_mesh_binary_u32(&mut bytes, secondary_uv_mask);
    for vertex in vertices {
        push_mesh_binary_f32s(&mut bytes, vertex);
    }
    for normal in normals {
        push_mesh_binary_f32s(&mut bytes, normal);
    }
    for uv in uvs {
        push_mesh_binary_f32s(&mut bytes, uv);
    }
    for uv_set in uv_sets {
        for uv in *uv_set {
            push_mesh_binary_f32s(&mut bytes, uv);
        }
    }
    for tangent in tangents {
        push_mesh_binary_f32s(&mut bytes, tangent);
    }
    for joint in joints {
        for value in joint {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for weight in weights {
        push_mesh_binary_f32s(&mut bytes, weight);
    }
    for index in indices {
        push_mesh_binary_u32(&mut bytes, *index);
    }
    bytes
}

fn invalid_binary_mesh_bytes() -> Vec<u8> {
    let mut bytes = b"NGA_MESH_BINARY_V1\n".to_vec();
    push_mesh_binary_u32(&mut bytes, 3);
    push_mesh_binary_u32(&mut bytes, 4);
    push_mesh_binary_u32(&mut bytes, 0);
    push_mesh_binary_u32(&mut bytes, 0);
    bytes
}

fn push_mesh_binary_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_mesh_binary_f32s<const N: usize>(bytes: &mut Vec<u8>, values: &[f32; N]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

fn shader_bytes() -> Vec<u8> {
    b"@fragment fn main() {}\n".to_vec()
}

fn model_manifest_bytes() -> Vec<u8> {
    b"NGA_MODEL_V1\nmesh=Mesh0|v 0 0 0;v 1 0 0;v 0 1 0;i 0 1 2\nmaterial=Material/Hero|name=hero;shader=shaders/pbr.wgsl;texture.albedo=textures/albedo.texture;base_color=1,1,1,1\nskeleton=Skeleton|NGA_SKELETON_V1;bone=Root\nanimation=Animation/Idle|NGA_ANIMATION_V1;duration=1;ticks_per_second=60;track=bone:Root;translation=0:0,0,0;rotation=0:0,0,0,1;scale=0:1,1,1\n".to_vec()
}

fn test_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/asset_database_tests")
        .join(format!("{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn database_config(name: &str) -> AssetDatabaseConfig {
    let root = test_dir(name);
    AssetDatabaseConfig {
        source_root: root.join("source"),
        imported_root: root.join("imported"),
        cooked_root: root.join("cooked"),
        registry_path: root.join("asset_registry.txt"),
    }
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
fn database_scans_imports_and_preserves_registry_identity() {
    let config = database_config("scan_import");
    let mut io = MemoryAssetIo::new();
    io.insert("textures/hero.texture", texture_bytes(1, 1, 200));

    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let scanned = database.scan().unwrap();
    assert_eq!(scanned, vec![AssetPath::parse("textures/hero.texture")]);

    let path = AssetPath::parse("textures/hero.texture");
    let first_id = database.import_asset_path(&path).unwrap();
    let second_id = database.import_asset_path(&path).unwrap();
    assert_eq!(first_id, second_id);

    let metadata = database.registry().get(first_id).unwrap();
    assert_eq!(metadata.path.as_ref(), Some(&path));
    assert_eq!(metadata.asset_type, AssetTypeId::of::<Texture>());
    assert_eq!(metadata.importer.as_deref(), Some("TextureImporter"));
    assert!(metadata.source_hash.is_some());
    assert!(config
        .imported_root
        .join("textures/hero.texture.meta")
        .exists());

    database.save_registry().unwrap();

    let mut loaded = AssetDatabase::new(config);
    loaded.load_registry().unwrap();
    assert_eq!(loaded.registry().id_from_path(&path), Some(first_id));
    assert_eq!(
        loaded.registry().get(first_id).unwrap().asset_type,
        AssetTypeId::of::<Texture>()
    );
}

#[test]
fn database_reports_missing_importer_as_import_error() {
    let config = database_config("missing_importer");
    let mut io = MemoryAssetIo::new();
    io.insert("data/blob.unknown", b"payload".to_vec());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);

    let error = database
        .import_asset_path(&AssetPath::parse("data/blob.unknown"))
        .unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("no importer registered for extension `unknown`")
    ));
}

struct FailingImporter;

impl AssetImporter for FailingImporter {
    fn name(&self) -> &'static str {
        "FailingImporter"
    }

    fn version(&self) -> u32 {
        1
    }

    fn extensions(&self) -> &[&'static str] {
        &["failimport"]
    }

    fn import(
        &self,
        _ctx: &mut ImportContext,
        _source: &SourceAsset,
        _settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError> {
        Err(AssetError::Import {
            message: "validation failed".to_owned(),
        })
    }
}

#[test]
fn database_importer_failures_include_source_importer_and_settings_context() {
    let config = database_config("importer_error_context");
    let path = AssetPath::parse("broken/asset.failimport");
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), b"bad".to_vec());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_importer(FailingImporter);
    let mut settings = ImporterSettings::default();
    settings.set("quality", "bad");
    settings.set("profile", "ci");

    let error = database
        .import_asset_path_with_settings(&path, &settings)
        .unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `FailingImporter` failed")
                && message.contains("broken/asset.failimport")
                && message.contains("profile=ci")
                && message.contains("quality=bad")
                && message.contains("validation failed")
    ));
}

struct TextureTestCooker;

impl AssetCooker for TextureTestCooker {
    fn name(&self) -> &'static str {
        "TextureTestCooker"
    }

    fn version(&self) -> u32 {
        3
    }

    fn asset_type(&self) -> AssetTypeId {
        AssetTypeId::of::<Texture>()
    }

    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        assert_eq!(ctx.target, TargetPlatform::Windows);
        assert!(!ctx.source_bytes.is_empty());
        Ok(CookOutput {
            id: metadata.id,
            bytes: vec![1, 2, 3, 4],
            content_hash: ContentHash(0x55aa),
            version_hash: VersionHash(self.version() as u64),
            metadata: metadata.clone(),
        })
    }
}

struct FailingTextureCooker;

impl AssetCooker for FailingTextureCooker {
    fn name(&self) -> &'static str {
        "FailingTextureCooker"
    }

    fn version(&self) -> u32 {
        1
    }

    fn asset_type(&self) -> AssetTypeId {
        AssetTypeId::of::<Texture>()
    }

    fn cook(&self, _ctx: &CookContext, _metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        Err(AssetError::Cook {
            message: "invalid texture payload".to_owned(),
        })
    }
}

#[test]
fn database_cook_asset_dispatches_by_asset_type_and_updates_hashes() {
    let config = database_config("cook");
    let mut io = MemoryAssetIo::new();
    io.insert("textures/hero.texture", texture_bytes(1, 1, 32));
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_cooker(TextureTestCooker);

    let id = database
        .import_asset_path(&AssetPath::parse("textures/hero.texture"))
        .unwrap();
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

    assert_eq!(output.id, id);
    assert_eq!(output.bytes, vec![1, 2, 3, 4]);
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(
        metadata.cooked_path.as_ref(),
        Some(&AssetPath::parse("textures/hero.texture"))
    );
    assert_eq!(metadata.cooked_hash, Some(ContentHash(0x55aa)));
    assert_eq!(metadata.version_hash, Some(VersionHash(3)));
    assert_eq!(
        fs::read(database.config().cooked_root.join("textures/hero.texture")).unwrap(),
        vec![1, 2, 3, 4]
    );
}

#[test]
fn database_missing_cooker_errors_include_asset_path_context() {
    let config = database_config("missing_cooker_context");
    let path = AssetPath::parse("textures/no_cooker.texture");
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), texture_bytes(1, 1, 12));
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();
    let id = database.import_asset_path(&path).unwrap();

    let error = database
        .cook_asset(id, TargetPlatform::Windows)
        .unwrap_err();

    assert!(matches!(
        error,
        AssetError::Cook { message }
            if message.contains("no cooker registered for asset type")
                && message.contains(&format!("{id:?}"))
                && message.contains("textures/no_cooker.texture")
    ));
}

#[test]
fn database_cooker_failures_include_asset_cooker_path_and_target_context() {
    let config = database_config("cooker_error_context");
    let path = AssetPath::parse("textures/bad_cook.texture");
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), texture_bytes(1, 1, 13));
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();
    database.register_cooker(FailingTextureCooker);
    let id = database.import_asset_path(&path).unwrap();

    let error = database.cook_asset(id, TargetPlatform::MacOs).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Cook { message }
            if message.contains("cooker `FailingTextureCooker` failed")
                && message.contains(&format!("{id:?}"))
                && message.contains("textures/bad_cook.texture")
                && message.contains("target MacOs")
                && message.contains("invalid texture payload")
    ));
}

#[test]
fn database_scan_reports_missing_metadata_for_new_sources() {
    let config = database_config("missing_meta");
    let mut io = MemoryAssetIo::new();
    io.insert("textures/new.texture", texture_bytes(1, 1, 77));
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let report = database.scan_with_metadata().unwrap();

    assert_eq!(
        report.sources,
        vec![AssetPath::parse("textures/new.texture")]
    );
    assert!(report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            AssetDatabaseDiagnostic::MissingMetadata { path }
                if *path == AssetPath::parse("textures/new.texture")
        )
    }));
}

#[test]
fn database_sidecar_rename_fallback_preserves_asset_id_for_moved_source() {
    let config = database_config("rename_fallback");
    let bytes = texture_bytes(1, 1, 99);
    let old_path = AssetPath::parse("textures/old_name.texture");
    let new_path = AssetPath::parse("textures/new_name.texture");

    let mut first_io = MemoryAssetIo::new();
    first_io.insert(old_path.path(), bytes.clone());
    let mut first = AssetDatabase::new(config.clone());
    first.set_io(first_io);
    first.register_builtin_importers();
    let old_id = first.import_asset_path(&old_path).unwrap();
    assert!(config
        .imported_root
        .join("textures/old_name.texture.meta")
        .exists());

    let mut moved_io = MemoryAssetIo::new();
    moved_io.insert(new_path.path(), bytes);
    let mut moved = AssetDatabase::new(config.clone());
    moved.set_io(moved_io);
    moved.register_builtin_importers();

    let report = moved.scan_with_metadata().unwrap();
    assert!(report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            AssetDatabaseDiagnostic::MovedSourcePath {
                id,
                old_path: old,
                new_path: new
            } if *id == old_id && *old == old_path && *new == new_path
        )
    }));
    assert_eq!(moved.registry().id_from_path(&new_path), Some(old_id));

    let imported_id = moved.import_asset_path(&new_path).unwrap();
    assert_eq!(imported_id, old_id);
    assert!(config
        .imported_root
        .join("textures/new_name.texture.meta")
        .exists());
}

#[test]
fn database_scan_reports_stale_metadata_when_source_is_missing() {
    let config = database_config("stale_meta");
    let path = AssetPath::parse("textures/gone.texture");
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), texture_bytes(1, 1, 11));
    let mut first = AssetDatabase::new(config.clone());
    first.set_io(io);
    first.register_builtin_importers();
    let id = first.import_asset_path(&path).unwrap();

    let mut second = AssetDatabase::new(config);
    second.set_io(MemoryAssetIo::new());
    second.register_builtin_importers();
    let report = second.scan_with_metadata().unwrap();

    assert!(report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            AssetDatabaseDiagnostic::StaleMetadata { id: stale_id, path: stale_path }
                if *stale_id == id && *stale_path == path
        )
    }));
}

#[test]
fn database_incremental_scan_classifies_added_changed_unchanged_and_removed_sources() {
    let config = database_config("incremental_scan");
    let unchanged_path = AssetPath::parse("textures/unchanged.texture");
    let changed_path = AssetPath::parse("textures/changed.texture");
    let removed_path = AssetPath::parse("textures/removed.texture");
    let added_path = AssetPath::parse("textures/added.texture");

    let mut first_io = MemoryAssetIo::new();
    first_io.insert(unchanged_path.path(), texture_bytes(1, 1, 1));
    first_io.insert(changed_path.path(), texture_bytes(1, 1, 2));
    first_io.insert(removed_path.path(), texture_bytes(1, 1, 3));
    let mut first = AssetDatabase::new(config.clone());
    first.set_io(first_io);
    first.register_builtin_importers();
    let unchanged_id = first.import_asset_path(&unchanged_path).unwrap();
    let changed_id = first.import_asset_path(&changed_path).unwrap();
    let removed_id = first.import_asset_path(&removed_path).unwrap();

    let mut second_io = MemoryAssetIo::new();
    second_io.insert(unchanged_path.path(), texture_bytes(1, 1, 1));
    second_io.insert(changed_path.path(), texture_bytes(2, 1, 4));
    second_io.insert(added_path.path(), texture_bytes(1, 1, 5));
    let mut second = AssetDatabase::new(config);
    second.set_io(second_io);
    second.register_builtin_importers();

    let report = second.scan_with_metadata().unwrap();

    assert_eq!(report.added, vec![added_path.clone()]);
    assert_eq!(report.changed, vec![changed_path.clone()]);
    assert_eq!(report.unchanged, vec![unchanged_path.clone()]);
    assert_eq!(report.removed, vec![removed_path.clone()]);
    assert_eq!(
        second.registry().id_from_path(&unchanged_path),
        Some(unchanged_id)
    );
    assert_eq!(
        second.registry().id_from_path(&changed_path),
        Some(changed_id)
    );
    assert_eq!(
        second.registry().id_from_path(&removed_path),
        Some(removed_id)
    );
    assert!(report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            AssetDatabaseDiagnostic::MissingMetadata { path } if *path == added_path
        )
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            AssetDatabaseDiagnostic::ChangedSource {
                id,
                path,
                previous_hash,
                current_hash,
            } if *id == changed_id && *path == changed_path && previous_hash != current_hash
        )
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            AssetDatabaseDiagnostic::StaleMetadata { id, path }
                if *id == removed_id && *path == removed_path
        )
    }));
}

#[test]
fn database_model_obj_context_mtl_changes_affect_source_hash_and_scan() {
    let config = database_config("model_obj_context_hash");
    let model_path = AssetPath::parse("models/context_hash.obj");
    let material_library_path = AssetPath::parse("models/context_hash.mtl");
    let material_path = AssetPath::parse("models/context_hash.Material_Red.material");
    let model_source = b"mtllib context_hash.mtl
o Prop
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
    .to_vec();

    let mut first_io = MemoryAssetIo::new();
    first_io.insert(model_path.path(), model_source.clone());
    first_io.insert(
        material_library_path.path(),
        b"newmtl Red\nKd 1 0 0\n".to_vec(),
    );
    let mut first = AssetDatabase::new(config.clone());
    first.set_io(first_io);
    first.register_builtin_importers();

    let model_id = first.import_asset_path(&model_path).unwrap();
    let first_hash = first.registry().get(model_id).unwrap().source_hash.unwrap();
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        b"# mtllib context_hash.mtl\nname=Red\nbase_color=1,0,0,1\n".to_vec()
    );
    first.save_all_metadata_sidecars().unwrap();

    let mut second_io = MemoryAssetIo::new();
    second_io.insert(model_path.path(), model_source);
    second_io.insert(
        material_library_path.path(),
        b"newmtl Red\nKd 0 1 0\n".to_vec(),
    );
    let mut second = AssetDatabase::new(config.clone());
    second.set_io(second_io);
    second.register_builtin_importers();

    let report = second.scan_with_metadata().unwrap();

    assert_eq!(report.changed, vec![model_path.clone()]);
    assert!(report.added.contains(&material_library_path));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            AssetDatabaseDiagnostic::ChangedSource {
                id,
                path,
                previous_hash,
                current_hash,
            } if *id == model_id
                && *path == model_path
                && *previous_hash == first_hash
                && previous_hash != current_hash
        )
    }));

    let reimported_id = second.import_asset_path(&model_path).unwrap();
    assert_eq!(reimported_id, model_id);
    assert_ne!(
        second
            .registry()
            .get(reimported_id)
            .unwrap()
            .source_hash
            .unwrap(),
        first_hash
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        b"# mtllib context_hash.mtl\nname=Red\nbase_color=0,1,0,1\n".to_vec()
    );
}

#[test]
fn database_sidecars_restore_dependency_metadata() {
    let config = database_config("dependency_restore");
    let mut database = AssetDatabase::new(config.clone());
    let texture_id = AssetId::new();
    let material_id = AssetId::new();
    let texture_path = AssetPath::parse("textures/albedo.texture");
    let material_path = AssetPath::parse("materials/hero.material");

    database.registry_mut().insert(AssetMetadata::runtime(
        texture_id,
        texture_path.clone(),
        AssetTypeId::of::<Texture>(),
    ));
    let mut material = AssetMetadata::runtime(
        material_id,
        material_path.clone(),
        AssetTypeId::of::<Material>(),
    );
    material.dependencies.push(texture_id);
    material.labels.push("hero".to_owned());
    database.registry_mut().insert(material);
    database.save_all_metadata_sidecars().unwrap();

    let mut loaded = AssetDatabase::new(config);
    let metadata = loaded.load_metadata_sidecars().unwrap();

    assert_eq!(metadata.len(), 2);
    let restored = loaded.registry().get(material_id).unwrap();
    assert_eq!(restored.path.as_ref(), Some(&material_path));
    assert_eq!(restored.dependencies, vec![texture_id]);
    assert_eq!(restored.labels, vec!["hero"]);
}

#[test]
fn database_dependency_report_saves_text_dot_json_and_html_exports() {
    let config = database_config("dependency_report_exports");
    fs::create_dir_all(&config.imported_root).unwrap();
    let mut database = AssetDatabase::new(config.clone());
    let texture_id = AssetId::new();
    let material_id = AssetId::new();
    let scene_id = AssetId::new();
    let texture_path = AssetPath::parse("textures/<albedo>&db.texture");
    let material_path = AssetPath::parse("materials/hero.material");
    let scene_path = AssetPath::parse("scenes/level.scene");

    database.registry_mut().insert(AssetMetadata::runtime(
        texture_id,
        texture_path,
        AssetTypeId::of::<Texture>(),
    ));
    let mut material =
        AssetMetadata::runtime(material_id, material_path, AssetTypeId::of::<Material>());
    material.dependencies.push(texture_id);
    database.registry_mut().insert(material);
    let mut scene = AssetMetadata::runtime(scene_id, scene_path, AssetTypeId::of::<SceneAsset>());
    scene.dependencies.push(material_id);
    database.registry_mut().insert(scene);

    let text_path = config.imported_root.join("dependencies.txt");
    let dot_path = config.imported_root.join("dependencies.dot");
    let json_path = config.imported_root.join("dependencies.json");
    let html_path = config.imported_root.join("dependencies.html");
    let scoped_json_path = config.imported_root.join("scene_dependencies.json");
    let scoped_html_path = config.imported_root.join("scene_dependencies.html");
    let scoped_text_path = config.imported_root.join("scene_dependencies.txt");
    let scoped_dot_path = config.imported_root.join("scene_dependencies.dot");
    database.save_dependency_report_text(&text_path).unwrap();
    database.save_dependency_report_dot(&dot_path).unwrap();
    database.save_dependency_report_json(&json_path).unwrap();
    database.save_dependency_report_html(&html_path).unwrap();
    database
        .save_scoped_dependency_report_text(scene_id, &scoped_text_path)
        .unwrap();
    database
        .save_scoped_dependency_report_dot(scene_id, &scoped_dot_path)
        .unwrap();
    database
        .save_scoped_dependency_report_json(scene_id, &scoped_json_path)
        .unwrap();
    database
        .save_scoped_dependency_report_html(scene_id, &scoped_html_path)
        .unwrap();

    let text = fs::read_to_string(text_path).unwrap();
    assert!(text.contains("NGA_DEPENDENCY_GRAPH_V1"));
    assert!(text.contains(&format!("asset|{}", texture_id.raw())));
    assert!(text.contains(&format!("edge|{}|{}", material_id.raw(), texture_id.raw())));
    assert!(text.contains(&format!("edge|{}|{}", scene_id.raw(), material_id.raw())));
    let dot = fs::read_to_string(dot_path).unwrap();
    assert!(dot.contains(&format!(
        "\"{}\" -> \"{}\";",
        material_id.raw(),
        texture_id.raw()
    )));
    let json = fs::read_to_string(json_path).unwrap();
    assert_eq!(
        json,
        format!(
            "{{\"version\":1,\"assets\":[\"{}\",\"{}\",\"{}\"],\"edges\":[{{\"asset\":\"{}\",\"dependency\":\"{}\"}},{{\"asset\":\"{}\",\"dependency\":\"{}\"}}]}}",
            texture_id.raw(),
            material_id.raw(),
            scene_id.raw(),
            material_id.raw(),
            texture_id.raw(),
            scene_id.raw(),
            material_id.raw()
        )
    );
    assert_eq!(database.dependency_report_json(), json);
    let html = fs::read_to_string(html_path).unwrap();
    assert_eq!(database.dependency_report_html(), html);
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("Asset Dependency Graph"));
    assert!(html.contains("textures/&lt;albedo&gt;&amp;db.texture"));
    assert!(!html.contains("textures/<albedo>&db.texture"));
    let scoped = database.scoped_dependency_report(scene_id).unwrap();
    assert_eq!(scoped.root, scene_id);
    assert_eq!(scoped.direct_dependencies, vec![material_id]);
    assert_eq!(
        scoped.transitive_dependencies,
        vec![texture_id, material_id]
    );
    assert!(scoped.graph.edges.contains(&DependencyEdge {
        asset: scene_id,
        dependency: material_id,
    }));
    assert!(scoped.graph.edges.contains(&DependencyEdge {
        asset: material_id,
        dependency: texture_id,
    }));
    let scoped_json = fs::read_to_string(scoped_json_path).unwrap();
    assert_eq!(
        database.scoped_dependency_report_json(scene_id).unwrap(),
        scoped_json
    );
    assert!(scoped_json.contains(&format!("\"root\":\"{}\"", scene_id.raw())));
    let scoped_html = fs::read_to_string(scoped_html_path).unwrap();
    assert_eq!(
        database.scoped_dependency_report_html(scene_id).unwrap(),
        scoped_html
    );
    assert!(scoped_html.contains("Asset Dependency Scope"));
    assert!(scoped_html.contains(&format!("data-root=\"{}\"", scene_id.raw())));
    assert!(scoped_html.contains(&format!("<code>{}</code>", material_id.raw())));
    let scoped_text = fs::read_to_string(scoped_text_path).unwrap();
    assert!(scoped_text.contains(&format!("asset|{}", scene_id.raw())));
    assert!(scoped_text.contains(&format!("edge|{}|{}", scene_id.raw(), material_id.raw())));
    let scoped_dot = fs::read_to_string(scoped_dot_path).unwrap();
    assert!(scoped_dot.contains(&format!(
        "\"{}\" -> \"{}\";",
        scene_id.raw(),
        material_id.raw()
    )));
    assert!(matches!(
        database.scoped_dependency_report(AssetId::from_u128(0xfeed_cafe)),
        Err(AssetError::AssetNotFound { .. })
    ));
    assert!(matches!(
        database.scoped_dependency_report_html(AssetId::from_u128(0xfeed_cafe)),
        Err(AssetError::AssetNotFound { .. })
    ));
    assert!(matches!(
        database.save_scoped_dependency_report_text(
            AssetId::from_u128(0xfeed_cafe),
            config.imported_root.join("missing_scope.txt")
        ),
        Err(AssetError::AssetNotFound { .. })
    ));
    assert!(matches!(
        database.save_scoped_dependency_report_dot(
            AssetId::from_u128(0xfeed_cafe),
            config.imported_root.join("missing_scope.dot")
        ),
        Err(AssetError::AssetNotFound { .. })
    ));
    assert!(matches!(
        database.save_scoped_dependency_report_json(
            AssetId::from_u128(0xfeed_cafe),
            config.imported_root.join("missing_scope.json")
        ),
        Err(AssetError::AssetNotFound { .. })
    ));
    assert!(matches!(
        database.save_scoped_dependency_report_html(
            AssetId::from_u128(0xfeed_cafe),
            config.imported_root.join("missing_scope.html")
        ),
        Err(AssetError::AssetNotFound { .. })
    ));
}

#[derive(Clone)]
struct DependencySubresourceImporter {
    dependency: AssetId,
    generated: AssetId,
    generated_path: AssetPath,
}

impl AssetImporter for DependencySubresourceImporter {
    fn name(&self) -> &'static str {
        "DependencySubresourceImporter"
    }

    fn version(&self) -> u32 {
        7
    }

    fn extensions(&self) -> &[&'static str] {
        &["sceneimp"]
    }

    fn import(
        &self,
        ctx: &mut ImportContext,
        source: &SourceAsset,
        _settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError> {
        let metadata = AssetMetadata::runtime(
            AssetId::new(),
            source.path.clone(),
            AssetTypeId::of::<SceneAsset>(),
        );
        ctx.add_dependency(self.dependency);
        ctx.add_generated_asset(ImportGeneratedAsset {
            id: self.generated,
            path: self.generated_path.clone(),
            asset_type: AssetTypeId::of::<Mesh>(),
            bytes: vec![1, 2, 3],
            labels: Vec::new(),
            dependencies: Vec::new(),
        });
        let (generated, dependencies) = std::mem::take(ctx).finish();
        Ok(ImportOutput {
            metadata,
            generated,
            dependencies,
            version_hash: VersionHash(7),
        })
    }
}

#[test]
fn importer_dependencies_and_generated_subresources_are_recorded_in_metadata() {
    let config = database_config("importer_dependencies_generated");
    let texture_path = AssetPath::parse("textures/linked.texture");
    let scene_path = AssetPath::parse("scenes/hero.sceneimp");
    let generated_path = AssetPath::parse("generated/hero_mesh.mesh");
    let generated_id = AssetId::new();
    let mut io = MemoryAssetIo::new();
    io.insert(texture_path.path(), texture_bytes(1, 1, 9));
    io.insert(scene_path.path(), b"scene".to_vec());

    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_importer(TextureImporter::new());
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    database.register_importer(DependencySubresourceImporter {
        dependency: texture_id,
        generated: generated_id,
        generated_path: generated_path.clone(),
    });

    let scene_id = database.import_asset_path(&scene_path).unwrap();
    let scene_metadata = database.registry().get(scene_id).unwrap();
    assert_eq!(scene_metadata.dependencies, vec![texture_id, generated_id]);
    assert_eq!(
        scene_metadata.importer.as_deref(),
        Some("DependencySubresourceImporter")
    );
    let generated_metadata = database
        .registry()
        .metadata_by_path(&generated_path)
        .unwrap();
    assert_eq!(generated_metadata.id, generated_id);
    assert_eq!(generated_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(generated_metadata.source_path.as_ref(), Some(&scene_path));
    assert_eq!(
        generated_metadata.cooked_path.as_ref(),
        Some(&generated_path)
    );
    assert_eq!(
        generated_metadata.importer.as_deref(),
        Some("DependencySubresourceImporter")
    );
    assert_eq!(generated_metadata.importer_version, 7);
    assert!(generated_metadata.source_hash.is_some());

    let report = database.dependency_report();
    assert!(report.edges.contains(&DependencyEdge {
        asset: scene_id,
        dependency: texture_id,
    }));
    assert!(report.edges.contains(&DependencyEdge {
        asset: scene_id,
        dependency: generated_id,
    }));
    assert!(database.dependency_report_text().contains(&format!(
        "edge|{}|{}",
        scene_id.raw(),
        texture_id.raw()
    )));
    assert!(database
        .dependency_report_html()
        .contains(&format!("<code>{}</code>", generated_id.raw())));

    database.save_registry().unwrap();
    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_registry = AssetDatabase::new(config.clone());
    loaded_registry.load_registry().unwrap();
    assert_eq!(
        loaded_registry
            .registry()
            .get(scene_id)
            .unwrap()
            .dependencies,
        vec![texture_id, generated_id]
    );
    assert_eq!(
        loaded_registry
            .registry()
            .metadata_by_path(&generated_path)
            .unwrap()
            .id,
        generated_id
    );

    let mut loaded_sidecars = AssetDatabase::new(config);
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&generated_path)
            .unwrap()
            .source_path
            .as_ref(),
        Some(&scene_path)
    );
}

#[test]
fn database_builtin_material_importer_records_dependencies_for_reports_sidecars_and_bundles() {
    let config = database_config("builtin_material_dependencies");
    let shader_path = AssetPath::parse("shaders/pbr.wgsl");
    let texture_path = AssetPath::parse("textures/albedo.texture");
    let material_path = AssetPath::parse("materials/hero.material");
    let shader_source = b"@fragment fn main() {}".to_vec();
    let texture_source = texture_bytes(1, 1, 88);
    let material_source =
        b"name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(shader_path.path(), shader_source.clone());
    io.insert(texture_path.path(), texture_source.clone());
    io.insert(material_path.path(), material_source.clone());

    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();
    let shader_id = database.import_asset_path(&shader_path).unwrap();
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    let material_id = database.import_asset_path(&material_path).unwrap();

    let material_metadata = database.registry().get(material_id).unwrap();
    assert_eq!(material_metadata.dependencies, vec![shader_id, texture_id]);

    let report = database.dependency_report();
    assert!(report.edges.contains(&DependencyEdge {
        asset: material_id,
        dependency: shader_id,
    }));
    assert!(report.edges.contains(&DependencyEdge {
        asset: material_id,
        dependency: texture_id,
    }));
    assert!(database.dependency_report_text().contains(&format!(
        "edge|{}|{}",
        material_id.raw(),
        texture_id.raw()
    )));
    assert!(database.dependency_report_dot().contains(&format!(
        "\"{}\" -> \"{}\"",
        material_id.raw(),
        shader_id.raw()
    )));

    database.save_registry().unwrap();
    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_registry = AssetDatabase::new(config.clone());
    loaded_registry.load_registry().unwrap();
    assert_eq!(
        loaded_registry
            .registry()
            .get(material_id)
            .unwrap()
            .dependencies,
        vec![shader_id, texture_id]
    );
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .get(material_id)
            .unwrap()
            .dependencies,
        vec![shader_id, texture_id]
    );

    database
        .cook_asset(shader_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(texture_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let output = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "materials",
            vec![material_id, shader_id, texture_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&output.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([shader_id, texture_id].as_slice())
    );
    assert_eq!(reader.read_path(&shader_path).unwrap(), shader_source);
    assert_eq!(reader.read_path(&texture_path).unwrap(), texture_source);
    assert_eq!(reader.read_path(&material_path).unwrap(), material_source);
}

#[test]
fn database_material_importer_canonicalizes_source_and_runtime_loads_it() {
    let config = database_config("material_importer_canonicalization");
    let shader_path = AssetPath::parse("shaders/pbr.wgsl");
    let texture_path = AssetPath::parse("textures/albedo.texture");
    let material_path = AssetPath::parse("materials/canonical.material");
    let source = b"# authoring comment\n name = hero \n\n shader = shaders/pbr.wgsl \n texture.albedo.source_channel = alpha \n texture.albedo = textures/albedo.texture \n base_color = 1, 0.5, 0.25, 1 \n roughness = 0.7 \n custom.tint.vec3 = 0.1, 0.2, 0.3 \n custom.illumination_model.int = 2 \n".to_vec();
    let canonical = b"name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo.source_channel=alpha\ntexture.albedo=textures/albedo.texture\nbase_color=1, 0.5, 0.25, 1\nroughness=0.7\ncustom.tint.vec3=0.1, 0.2, 0.3\ncustom.illumination_model.int=2\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(shader_path.path(), shader_bytes());
    io.insert(texture_path.path(), texture_bytes(1, 1, 60));
    io.insert(material_path.path(), source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let shader_id = database.import_asset_path(&shader_path).unwrap();
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    let material_id = database.import_asset_path(&material_path).unwrap();
    let metadata = database.registry().get(material_id).unwrap();
    assert_eq!(metadata.importer.as_deref(), Some("MaterialImporter"));
    assert_eq!(metadata.importer_version, 5);
    assert_eq!(metadata.dependencies, vec![shader_id, texture_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        canonical
    );

    let output = database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(shader_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(texture_id, TargetPlatform::Windows)
        .unwrap();
    assert_eq!(output.bytes, canonical);
    assert_eq!(
        fs::read(config.cooked_root.join(material_path.path())).unwrap(),
        canonical
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let shader: Handle<Shader> = server.load(shader_path);
    let texture: Handle<Texture> = server.load(texture_path);
    let material: Handle<Material> = server.load(material_path);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.is_ready(&material) {
            break;
        }
    }

    assert!(server.is_ready(&shader));
    assert!(server.is_ready(&texture));
    assert!(server.is_ready_with_dependencies(&material));
    let loaded = server.get(&material).unwrap();
    assert_eq!(loaded.name.as_deref(), Some("hero"));
    assert_eq!(
        loaded.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Alpha)
    );
    assert_eq!(loaded.properties.base_color, [1.0, 0.5, 0.25, 1.0]);
    assert_eq!(loaded.properties.roughness, 0.7);
    assert_eq!(
        loaded.properties.custom.get("tint"),
        Some(&MaterialPropertyValue::Vec3([0.1, 0.2, 0.3]))
    );
    assert_eq!(
        loaded.properties.custom.get("illumination_model"),
        Some(&MaterialPropertyValue::Int(2))
    );
}

#[test]
fn database_material_importer_preserves_texture_metadata_round_trip() {
    let config = database_config("material_importer_texture_metadata_round_trip");
    let shader_path = AssetPath::parse("shaders/pbr.wgsl");
    let albedo_path = AssetPath::parse("textures/albedo.texture");
    let normal_path = AssetPath::parse("textures/normal.texture");
    let material_path = AssetPath::parse("materials/textured.material");
    let source = b"# authoring comment\n name = hero \n\n shader = shaders/pbr.wgsl \n texture.albedo = textures/albedo.texture \n texture.albedo.source_channel = alpha \n texture.albedo.boost = 1.5 \n texture.albedo.color_correction = true \n texture.albedo.color_space = linear \n texture.normal = textures/normal.texture \n texture.normal.source_channel = red \n texture.normal.bump_scale = 0.3 \n texture.normal.color_space = non_color \n base_color = 1, 0.5, 0.25, 1 \n roughness = 0.7 \n"
        .to_vec();
    let canonical = b"name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\ntexture.albedo.source_channel=alpha\ntexture.albedo.boost=1.5\ntexture.albedo.color_correction=true\ntexture.albedo.color_space=linear\ntexture.normal=textures/normal.texture\ntexture.normal.source_channel=red\ntexture.normal.bump_scale=0.3\ntexture.normal.color_space=non_color\nbase_color=1, 0.5, 0.25, 1\nroughness=0.7\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(shader_path.path(), shader_bytes());
    io.insert(albedo_path.path(), texture_bytes(1, 1, 61));
    io.insert(normal_path.path(), texture_bytes(1, 1, 62));
    io.insert(material_path.path(), source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let shader_id = database.import_asset_path(&shader_path).unwrap();
    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let normal_id = database.import_asset_path(&normal_path).unwrap();
    let material_id = database.import_asset_path(&material_path).unwrap();
    let metadata = database.registry().get(material_id).unwrap();
    assert_eq!(metadata.importer.as_deref(), Some("MaterialImporter"));
    assert_eq!(metadata.importer_version, 5);
    assert_eq!(metadata.dependencies, vec![shader_id, albedo_id, normal_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        canonical
    );

    let output = database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(shader_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(normal_id, TargetPlatform::Windows)
        .unwrap();
    assert_eq!(output.bytes, canonical);
    assert_eq!(
        fs::read(config.cooked_root.join(material_path.path())).unwrap(),
        canonical
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let shader: Handle<Shader> = server.load(shader_path);
    let albedo: Handle<Texture> = server.load(albedo_path);
    let normal: Handle<Texture> = server.load(normal_path);
    let material: Handle<Material> = server.load(material_path);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.is_ready(&material) {
            break;
        }
    }

    assert!(server.is_ready(&shader));
    assert!(server.is_ready(&albedo));
    assert!(server.is_ready(&normal));
    assert!(server.is_ready_with_dependencies(&material));
    let loaded = server.get(&material).unwrap();
    assert_eq!(loaded.name.as_deref(), Some("hero"));
    assert_eq!(
        loaded.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Alpha)
    );
    assert_eq!(loaded.textures[0].options.boost, Some(1.5));
    assert_eq!(loaded.textures[0].options.color_correction, Some(true));
    assert_eq!(
        loaded.textures[0].options.color_space,
        Some(MaterialTextureColorSpace::Linear)
    );
    assert_eq!(
        loaded.textures[1].options.source_channel,
        Some(MaterialTextureChannel::Red)
    );
    assert_eq!(loaded.textures[1].options.bump_scale, Some(0.3));
    assert_eq!(
        loaded.textures[1].options.color_space,
        Some(MaterialTextureColorSpace::NonColor)
    );
    assert_eq!(loaded.properties.base_color, [1.0, 0.5, 0.25, 1.0]);
    assert_eq!(loaded.properties.roughness, 0.7);
}

#[test]
fn database_material_cooker_canonicalizes_runtime_source_bytes() {
    let material_path = AssetPath::parse("materials/cooked.material");
    let source = b"# comment\n name = hero \n shader = shaders/pbr.wgsl \n texture.albedo = textures/albedo.texture \n base_color = 1, 0.5, 0.25, 1 \n roughness = 0.7 \n".to_vec();
    let expected = b"name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=1, 0.5, 0.25, 1\nroughness=0.7\n".to_vec();
    let ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(material_path.clone()),
        source_bytes: source,
    };
    let metadata =
        AssetMetadata::runtime(AssetId::new(), material_path, AssetTypeId::of::<Material>());
    let cooker = MaterialCooker::new();

    let output = cooker.cook(&ctx, &metadata).unwrap();

    assert_eq!(output.bytes, expected);
    assert_eq!(output.version_hash, VersionHash(2));
}

#[test]
fn database_material_cooker_canonicalizes_runtime_and_source_bytes() {
    let runtime_path = AssetPath::parse("materials/cooked_runtime.material");
    let runtime_bytes = b"name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=1, 0.5, 0.25, 1\nroughness=0.7\n".to_vec();
    let runtime_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(runtime_path.clone()),
        source_bytes: runtime_bytes.clone(),
    };
    let runtime_metadata = AssetMetadata::runtime(
        AssetId::new(),
        runtime_path,
        AssetTypeId::of::<Material>(),
    );
    let source_path = AssetPath::parse("materials/from_source.material");
    let source_bytes = b"# comment\n name = hero \n shader = shaders/pbr.wgsl \n texture.albedo = textures/albedo.texture \n base_color = 1, 0.5, 0.25, 1 \n roughness = 0.7 \n".to_vec();
    let source_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(source_path.clone()),
        source_bytes: source_bytes.clone(),
    };
    let source_metadata = AssetMetadata::runtime(
        AssetId::new(),
        source_path,
        AssetTypeId::of::<Material>(),
    );
    let cooker = MaterialCooker::new();
    let expected = b"name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=1, 0.5, 0.25, 1\nroughness=0.7\n".to_vec();

    let runtime_output = cooker.cook(&runtime_ctx, &runtime_metadata).unwrap();
    let source_output = cooker.cook(&source_ctx, &source_metadata).unwrap();

    assert_eq!(runtime_output.bytes, expected);
    assert_eq!(runtime_output.version_hash, VersionHash(2));
    assert_eq!(runtime_output.metadata, runtime_metadata);
    assert_eq!(source_output.bytes, expected);
    assert_eq!(source_output.version_hash, VersionHash(2));
    assert_eq!(source_output.metadata, source_metadata);
}

#[test]
fn database_scene_and_prefab_cookers_pass_through_runtime_bytes() {
    let scene_path = AssetPath::parse("scenes/cooked.scene");
    let prefab_path = AssetPath::parse("prefabs/cooked.prefab");
    let scene_source =
        b"NGA_SCENE_V1\nname=cooked_scene\ndependency=textures/albedo.texture\n".to_vec();
    let prefab_source =
        b"NGA_PREFAB_V1\ndependency=textures/albedo.texture\nroot=Root\n".to_vec();
    let scene_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(scene_path.clone()),
        source_bytes: scene_source.clone(),
    };
    let prefab_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(prefab_path.clone()),
        source_bytes: prefab_source.clone(),
    };
    let scene_metadata =
        AssetMetadata::runtime(AssetId::new(), scene_path, AssetTypeId::of::<SceneAsset>());
    let prefab_metadata =
        AssetMetadata::runtime(AssetId::new(), prefab_path, AssetTypeId::of::<Prefab>());
    let scene_cooker = SceneCooker::new();
    let prefab_cooker = PrefabCooker::new();

    let scene_output = scene_cooker.cook(&scene_ctx, &scene_metadata).unwrap();
    let prefab_output = prefab_cooker.cook(&prefab_ctx, &prefab_metadata).unwrap();

    assert_eq!(scene_output.bytes, scene_source);
    assert_eq!(scene_output.version_hash, VersionHash(1));
    assert_eq!(prefab_output.bytes, prefab_source);
    assert_eq!(prefab_output.version_hash, VersionHash(1));
}

#[test]
fn database_font_and_physics_mesh_cookers_pass_through_runtime_bytes() {
    let font_path = AssetPath::parse("fonts/cooked.font");
    let physics_path = AssetPath::parse("physics/cooked.physics");
    let font_source = font_bytes();
    let physics_source = physics_mesh_bytes();
    let font_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(font_path.clone()),
        source_bytes: font_source.clone(),
    };
    let physics_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(physics_path.clone()),
        source_bytes: physics_source.clone(),
    };
    let font_metadata =
        AssetMetadata::runtime(AssetId::new(), font_path, AssetTypeId::of::<Font>());
    let physics_metadata = AssetMetadata::runtime(
        AssetId::new(),
        physics_path,
        AssetTypeId::of::<PhysicsMesh>(),
    );
    let font_cooker = FontCooker::new();
    let physics_cooker = PhysicsMeshCooker::new();

    let font_output = font_cooker.cook(&font_ctx, &font_metadata).unwrap();
    let physics_output = physics_cooker.cook(&physics_ctx, &physics_metadata).unwrap();

    assert_eq!(font_output.bytes, font_source);
    assert_eq!(font_output.version_hash, VersionHash(2));
    assert_eq!(font_output.metadata, font_metadata);
    assert_eq!(physics_output.bytes, physics_source);
    assert_eq!(physics_output.version_hash, VersionHash(1));
    assert_eq!(physics_output.metadata, physics_metadata);
}

#[test]
fn database_font_cooker_canonicalizes_runtime_and_source_bytes() {
    let runtime_path = AssetPath::parse("fonts/cooked_runtime.font");
    let runtime_bytes = font_bytes();
    let runtime_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(runtime_path.clone()),
        source_bytes: runtime_bytes.clone(),
    };
    let runtime_metadata =
        AssetMetadata::runtime(AssetId::new(), runtime_path, AssetTypeId::of::<Font>());
    let source_path = AssetPath::parse("fonts/from_source.font");
    let source_bytes =
        b"NGA_FONT_V1\nfamily=Mono\nglyph=char=A;size=1x1;bitmap=255\n".to_vec();
    let source_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(source_path.clone()),
        source_bytes: source_bytes.clone(),
    };
    let source_metadata =
        AssetMetadata::runtime(AssetId::new(), source_path, AssetTypeId::of::<Font>());
    let cooker = FontCooker::new();

    let runtime_output = cooker.cook(&runtime_ctx, &runtime_metadata).unwrap();
    let source_output = cooker.cook(&source_ctx, &source_metadata).unwrap();

    assert_eq!(runtime_output.bytes, runtime_bytes);
    assert_eq!(runtime_output.version_hash, VersionHash(2));
    assert_eq!(runtime_output.metadata, runtime_metadata);
    assert_eq!(source_output.bytes, source_bytes);
    assert_eq!(source_output.version_hash, VersionHash(2));
    assert_eq!(source_output.metadata, source_metadata);
}

#[test]
fn database_scene_prefab_and_physics_mesh_cookers_pass_through_runtime_and_source_bytes() {
    let scene_path = AssetPath::parse("scenes/cooked_runtime.scene");
    let prefab_path = AssetPath::parse("prefabs/cooked_runtime.prefab");
    let physics_path = AssetPath::parse("physics/cooked_runtime.physics");
    let scene_runtime_bytes =
        b"NGA_SCENE_V1\nname=runtime_scene\ndependency=textures/albedo.texture\n".to_vec();
    let prefab_runtime_bytes =
        b"NGA_PREFAB_V1\ndependency=textures/albedo.texture\nroot=Root\n".to_vec();
    let physics_runtime_bytes = physics_mesh_bytes();
    let scene_runtime_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(scene_path.clone()),
        source_bytes: scene_runtime_bytes.clone(),
    };
    let prefab_runtime_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(prefab_path.clone()),
        source_bytes: prefab_runtime_bytes.clone(),
    };
    let physics_runtime_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(physics_path.clone()),
        source_bytes: physics_runtime_bytes.clone(),
    };
    let scene_runtime_metadata =
        AssetMetadata::runtime(AssetId::new(), scene_path, AssetTypeId::of::<SceneAsset>());
    let prefab_runtime_metadata =
        AssetMetadata::runtime(AssetId::new(), prefab_path, AssetTypeId::of::<Prefab>());
    let physics_runtime_metadata = AssetMetadata::runtime(
        AssetId::new(),
        physics_path,
        AssetTypeId::of::<PhysicsMesh>(),
    );
    let scene_source_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(AssetPath::parse("scenes/from_source.scene")),
        source_bytes: scene_runtime_bytes.clone(),
    };
    let prefab_source_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(AssetPath::parse("prefabs/from_source.prefab")),
        source_bytes: prefab_runtime_bytes.clone(),
    };
    let physics_source_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(AssetPath::parse("physics/from_source.physics")),
        source_bytes: physics_runtime_bytes.clone(),
    };
    let scene_source_metadata = AssetMetadata::runtime(
        AssetId::new(),
        AssetPath::parse("scenes/from_source.scene"),
        AssetTypeId::of::<SceneAsset>(),
    );
    let prefab_source_metadata = AssetMetadata::runtime(
        AssetId::new(),
        AssetPath::parse("prefabs/from_source.prefab"),
        AssetTypeId::of::<Prefab>(),
    );
    let physics_source_metadata = AssetMetadata::runtime(
        AssetId::new(),
        AssetPath::parse("physics/from_source.physics"),
        AssetTypeId::of::<PhysicsMesh>(),
    );
    let scene_cooker = SceneCooker::new();
    let prefab_cooker = PrefabCooker::new();
    let physics_cooker = PhysicsMeshCooker::new();

    let scene_runtime_output = scene_cooker
        .cook(&scene_runtime_ctx, &scene_runtime_metadata)
        .unwrap();
    let prefab_runtime_output = prefab_cooker
        .cook(&prefab_runtime_ctx, &prefab_runtime_metadata)
        .unwrap();
    let physics_runtime_output = physics_cooker
        .cook(&physics_runtime_ctx, &physics_runtime_metadata)
        .unwrap();
    let scene_source_output = scene_cooker
        .cook(&scene_source_ctx, &scene_source_metadata)
        .unwrap();
    let prefab_source_output = prefab_cooker
        .cook(&prefab_source_ctx, &prefab_source_metadata)
        .unwrap();
    let physics_source_output = physics_cooker
        .cook(&physics_source_ctx, &physics_source_metadata)
        .unwrap();

    assert_eq!(scene_runtime_output.bytes, scene_runtime_bytes);
    assert_eq!(scene_runtime_output.version_hash, VersionHash(1));
    assert_eq!(scene_runtime_output.metadata, scene_runtime_metadata);
    assert_eq!(prefab_runtime_output.bytes, prefab_runtime_bytes);
    assert_eq!(prefab_runtime_output.version_hash, VersionHash(1));
    assert_eq!(prefab_runtime_output.metadata, prefab_runtime_metadata);
    assert_eq!(physics_runtime_output.bytes, physics_runtime_bytes);
    assert_eq!(physics_runtime_output.version_hash, VersionHash(1));
    assert_eq!(physics_runtime_output.metadata, physics_runtime_metadata);
    assert_eq!(scene_source_output.bytes, scene_runtime_bytes);
    assert_eq!(scene_source_output.version_hash, VersionHash(1));
    assert_eq!(scene_source_output.metadata, scene_source_metadata);
    assert_eq!(prefab_source_output.bytes, prefab_runtime_bytes);
    assert_eq!(prefab_source_output.version_hash, VersionHash(1));
    assert_eq!(prefab_source_output.metadata, prefab_source_metadata);
    assert_eq!(physics_source_output.bytes, physics_runtime_bytes);
    assert_eq!(physics_source_output.version_hash, VersionHash(1));
    assert_eq!(physics_source_output.metadata, physics_source_metadata);
}

#[test]
fn database_shader_cooker_canonicalizes_source_documents() {
    let shader_path = AssetPath::parse("shaders/cooked.wgsl");
    let source =
        b"NGA_SHADER_SOURCE_V1\nlanguage=wgsl\nstage=fragment\n---\n  @fragment fn main() {}\n"
            .to_vec();
    let expected = b"@fragment fn main() {}\n".to_vec();
    let ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(shader_path.clone()),
        source_bytes: source,
    };
    let metadata = AssetMetadata::runtime(AssetId::new(), shader_path, AssetTypeId::of::<Shader>());
    let cooker = ShaderCooker::new();

    let output = cooker.cook(&ctx, &metadata).unwrap();

    assert_eq!(output.bytes, expected);
    assert_eq!(output.version_hash, VersionHash(2));
}

#[test]
fn database_shader_cooker_canonicalizes_runtime_and_source_bytes() {
    let runtime_path = AssetPath::parse("shaders/cooked_runtime.wgsl");
    let runtime_bytes = b"@fragment fn main() {}\n".to_vec();
    let runtime_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(runtime_path.clone()),
        source_bytes: runtime_bytes.clone(),
    };
    let runtime_metadata = AssetMetadata::runtime(
        AssetId::new(),
        runtime_path,
        AssetTypeId::of::<Shader>(),
    );
    let source_path = AssetPath::parse("shaders/from_source.wgsl");
    let source_bytes =
        b"NGA_SHADER_SOURCE_V1\nlanguage=wgsl\nstage=fragment\n---\n  @fragment fn main() {}\n"
            .to_vec();
    let source_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(source_path.clone()),
        source_bytes: source_bytes.clone(),
    };
    let source_metadata =
        AssetMetadata::runtime(AssetId::new(), source_path, AssetTypeId::of::<Shader>());
    let cooker = ShaderCooker::new();
    let expected = b"@fragment fn main() {}\n".to_vec();

    let runtime_output = cooker.cook(&runtime_ctx, &runtime_metadata).unwrap();
    let source_output = cooker.cook(&source_ctx, &source_metadata).unwrap();

    assert_eq!(runtime_output.bytes, expected);
    assert_eq!(runtime_output.version_hash, VersionHash(2));
    assert_eq!(runtime_output.metadata, runtime_metadata);
    assert_eq!(source_output.bytes, expected);
    assert_eq!(source_output.version_hash, VersionHash(2));
    assert_eq!(source_output.metadata, source_metadata);
}

#[test]
fn database_builtin_material_importer_reports_missing_dependency_path() {
    let config = database_config("builtin_material_missing_dependency");
    let material_path = AssetPath::parse("materials/missing_shader.material");
    let mut io = MemoryAssetIo::new();
    io.insert(
        material_path.path(),
        b"name=broken\nshader=shaders/missing.wgsl\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&material_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `MaterialImporter` failed")
                && message.contains("material dependency `shader`")
                && message.contains("shaders/missing.wgsl")
                && message.contains("not registered")
    ));
}

#[test]
fn database_builtin_model_importer_reports_missing_material_dependency_path() {
    let config = database_config("builtin_model_missing_dependency");
    let model_path = AssetPath::parse("models/hero.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmaterial=Material/Hero|name=hero;shader=shaders/missing.wgsl\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("material dependency `shader`")
                && message.contains("shaders/missing.wgsl")
                && message.contains("not registered")
    ));
}

#[test]
#[cfg(feature = "bundle")]
fn database_builtin_model_importer_generates_labeled_subresources_and_runtime_outputs() {
    let config = database_config("builtin_model_generated_subresources");
    let shader_path = AssetPath::parse("shaders/pbr.wgsl");
    let texture_path = AssetPath::parse("textures/albedo.texture");
    let model_path = AssetPath::parse("models/hero.model");
    let mesh_path = AssetPath::parse("models/hero.Mesh0.mesh");
    let material_path = AssetPath::parse("models/hero.Material_Hero.material");
    let skeleton_path = AssetPath::parse("models/hero.Skeleton.skeleton");
    let animation_path = AssetPath::parse("models/hero.Animation_Idle.animation");
    let material_bytes =
        b"name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=1,1,1,1\n"
            .to_vec();
    let skeleton_bytes = b"NGA_SKELETON_V1\nbone=Root\n".to_vec();
    let animation_bytes = b"NGA_ANIMATION_V1\nduration=1\nticks_per_second=60\ntrack=bone:Root\ntranslation=0:0,0,0\nrotation=0:0,0,0,1\nscale=0:1,1,1\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(shader_path.path(), shader_bytes());
    io.insert(texture_path.path(), texture_bytes(1, 1, 44));
    io.insert(model_path.path(), model_manifest_bytes());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let shader_id = database.import_asset_path(&shader_path).unwrap();
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let model_metadata = database.registry().get(model_id).unwrap();
    assert_eq!(model_metadata.asset_type, AssetTypeId::NIL);

    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;
    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.source_path.as_ref(), Some(&model_path));
    assert_eq!(mesh_metadata.cooked_path.as_ref(), Some(&mesh_path));
    assert_eq!(mesh_metadata.labels, vec!["Mesh0"]);
    assert_eq!(mesh_metadata.importer.as_deref(), Some("ModelImporter"));
    assert!(config.imported_root.join(mesh_path.path()).exists());

    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;
    assert_eq!(material_metadata.asset_type, AssetTypeId::of::<Material>());
    assert_eq!(material_metadata.dependencies, vec![shader_id, texture_id]);
    assert_eq!(material_metadata.labels, vec!["Material/Hero"]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        material_bytes
    );
    let skeleton_metadata = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap();
    let skeleton_id = skeleton_metadata.id;
    assert_eq!(skeleton_metadata.asset_type, AssetTypeId::of::<Skeleton>());
    assert_eq!(skeleton_metadata.labels, vec!["Skeleton"]);
    assert_eq!(
        fs::read(config.imported_root.join(skeleton_path.path())).unwrap(),
        skeleton_bytes
    );
    let animation_metadata = database
        .registry()
        .metadata_by_path(&animation_path)
        .unwrap();
    let animation_id = animation_metadata.id;
    assert_eq!(
        animation_metadata.asset_type,
        AssetTypeId::of::<AnimationClip>()
    );
    assert_eq!(animation_metadata.labels, vec!["Animation/Idle"]);
    assert_eq!(
        fs::read(config.imported_root.join(animation_path.path())).unwrap(),
        animation_bytes
    );
    let model_metadata = database.registry().get(model_id).unwrap();
    assert_eq!(
        model_metadata.dependencies,
        vec![
            shader_id,
            texture_id,
            mesh_id,
            material_id,
            skeleton_id,
            animation_id
        ]
    );

    let report = database.dependency_report();
    assert!(report.edges.contains(&DependencyEdge {
        asset: model_id,
        dependency: shader_id,
    }));
    assert!(report.edges.contains(&DependencyEdge {
        asset: model_id,
        dependency: mesh_id,
    }));
    assert!(report.edges.contains(&DependencyEdge {
        asset: model_id,
        dependency: material_id,
    }));
    assert!(report.edges.contains(&DependencyEdge {
        asset: model_id,
        dependency: skeleton_id,
    }));
    assert!(report.edges.contains(&DependencyEdge {
        asset: model_id,
        dependency: animation_id,
    }));
    assert!(report.edges.contains(&DependencyEdge {
        asset: material_id,
        dependency: texture_id,
    }));
    assert!(database.dependency_report_text().contains(&format!(
        "edge|{}|{}",
        model_id.raw(),
        skeleton_id.raw()
    )));
    assert!(database.dependency_report_text().contains(&format!(
        "edge|{}|{}",
        model_id.raw(),
        animation_id.raw()
    )));
    assert!(database
        .dependency_report_json()
        .contains(&format!("\"{}\"", skeleton_id.raw())));
    assert!(database
        .dependency_report_json()
        .contains(&format!("\"{}\"", animation_id.raw())));
    assert!(database
        .dependency_report_html()
        .contains(&format!("<code>{}</code>", skeleton_id.raw())));
    assert!(database
        .dependency_report_html()
        .contains(&format!("<code>{}</code>", animation_id.raw())));
    let model_scope = database.scoped_dependency_report(model_id).unwrap();
    assert!(model_scope.direct_dependencies.contains(&material_id));
    assert!(model_scope.direct_dependencies.contains(&skeleton_id));
    assert!(model_scope.direct_dependencies.contains(&animation_id));
    assert!(model_scope.transitive_dependencies.contains(&texture_id));
    assert!(database
        .scoped_dependency_report_json(model_id)
        .unwrap()
        .contains(&format!("\"{}\"", skeleton_id.raw())));
    assert!(database
        .scoped_dependency_report_json(model_id)
        .unwrap()
        .contains(&format!("\"{}\"", animation_id.raw())));
    assert!(database
        .scoped_dependency_report_html(model_id)
        .unwrap()
        .contains(&format!("<code>{}</code>", skeleton_id.raw())));
    assert!(database
        .scoped_dependency_report_html(model_id)
        .unwrap()
        .contains(&format!("<code>{}</code>", animation_id.raw())));
    assert!(database
        .scoped_dependency_report_html(model_id)
        .unwrap()
        .contains("models/hero.Material_Hero.material"));

    let text_path = config.imported_root.join("model_dependencies.txt");
    let json_path = config.imported_root.join("model_dependencies.json");
    let html_path = config.imported_root.join("model_dependencies.html");
    let scoped_json_path = config.imported_root.join("model_scope.json");
    let scoped_html_path = config.imported_root.join("model_scope.html");
    database.save_dependency_report_text(&text_path).unwrap();
    database.save_dependency_report_json(&json_path).unwrap();
    database.save_dependency_report_html(&html_path).unwrap();
    database
        .save_scoped_dependency_report_json(model_id, &scoped_json_path)
        .unwrap();
    database
        .save_scoped_dependency_report_html(model_id, &scoped_html_path)
        .unwrap();

    let text = fs::read_to_string(text_path).unwrap();
    assert!(text.contains(&format!("edge|{}|{}", model_id.raw(), skeleton_id.raw())));
    assert!(text.contains(&format!("edge|{}|{}", model_id.raw(), animation_id.raw())));
    let json = fs::read_to_string(json_path).unwrap();
    assert!(json.contains(&format!("\"{}\"", skeleton_id.raw())));
    assert!(json.contains(&format!("\"{}\"", animation_id.raw())));
    let html = fs::read_to_string(html_path).unwrap();
    assert!(html.contains(&format!("<code>{}</code>", skeleton_id.raw())));
    assert!(html.contains(&format!("<code>{}</code>", animation_id.raw())));
    let scoped_json = fs::read_to_string(scoped_json_path).unwrap();
    assert!(scoped_json.contains(&format!("\"{}\"", skeleton_id.raw())));
    assert!(scoped_json.contains(&format!("\"{}\"", animation_id.raw())));
    let scoped_html = fs::read_to_string(scoped_html_path).unwrap();
    assert!(scoped_html.contains(&format!("<code>{}</code>", skeleton_id.raw())));
    assert!(scoped_html.contains(&format!("<code>{}</code>", animation_id.raw())));

    database.save_registry().unwrap();
    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .get(model_id)
            .unwrap()
            .dependencies,
        vec![
            shader_id,
            texture_id,
            mesh_id,
            material_id,
            skeleton_id,
            animation_id
        ]
    );
    let reloaded_material = loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    assert_eq!(reloaded_material.dependencies, vec![shader_id, texture_id]);
    assert_eq!(reloaded_material.labels, vec!["Material/Hero"]);

    database
        .cook_asset(shader_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(texture_id, TargetPlatform::Windows)
        .unwrap();
    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let material_output = database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let skeleton_output = database
        .cook_asset(skeleton_id, TargetPlatform::Windows)
        .unwrap();
    let animation_output = database
        .cook_asset(animation_id, TargetPlatform::Windows)
        .unwrap();
    let expected_cooked_mesh = simple_binary_mesh_bytes();
    assert_eq!(mesh_output.bytes, expected_cooked_mesh);
    assert_eq!(mesh_output.version_hash, VersionHash(4));
    assert_eq!(material_output.bytes, material_bytes);
    assert_eq!(skeleton_output.bytes, skeleton_bytes);
    assert_eq!(animation_output.bytes, animation_bytes);

    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "hero_model",
            vec![
                mesh_id,
                material_id,
                skeleton_id,
                animation_id,
                shader_id,
                texture_id,
            ],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([shader_id, texture_id].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), expected_cooked_mesh);
    assert_eq!(
        reader.read_path(&material_path).unwrap(),
        material_output.bytes
    );
    assert_eq!(
        reader.read_path(&skeleton_path).unwrap(),
        skeleton_output.bytes
    );
    assert_eq!(
        reader.read_path(&animation_path).unwrap(),
        animation_output.bytes
    );

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(mesh_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(material_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(skeleton_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(animation_id), AssetLoadState::Ready);
    assert_eq!(
        server.get_by_id::<Skeleton>(skeleton_id).unwrap().bones[0].name,
        "Root"
    );
    assert_eq!(
        server
            .get_by_id::<AnimationClip>(animation_id)
            .unwrap()
            .tracks[0]
            .target,
        AnimationTarget::BoneName("Root".to_owned())
    );
}

#[test]
fn database_model_importer_parses_blocks_and_remaps_generated_dependency_ids() {
    let config = database_config("builtin_model_block_manifest");
    let shader_path = AssetPath::parse("shaders/pbr.wgsl");
    let texture_path = AssetPath::parse("textures/albedo.texture");
    let model_path = AssetPath::parse("models/block.model");
    let mesh_path = AssetPath::parse("models/block.Body.mesh");
    let skeleton_path = AssetPath::parse("models/block.Rig.skeleton");
    let animation_path = AssetPath::parse("models/block.Walk.animation");
    let material_path = AssetPath::parse("models/block.HeroMaterial.material");
    let model_source = b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\nanimation=Walk\ndepends=Rig\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=node:Root\ntranslation=0:0,0,0\nrotation=0:0,0,0,1\nscale=0:1,1,1\nend\nmaterial=HeroMaterial\ndepends=Body\nname=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=0.8,0.7,0.6,1\nend\n".to_vec();
    let expected_mesh = mesh_bytes();
    let expected_skeleton = b"NGA_SKELETON_V1\nbone=Root\n".to_vec();
    let expected_animation = b"NGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=node:Root\ntranslation=0:0,0,0\nrotation=0:0,0,0,1\nscale=0:1,1,1\n".to_vec();
    let expected_material = b"name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=0.8,0.7,0.6,1\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(shader_path.path(), shader_bytes());
    io.insert(texture_path.path(), texture_bytes(1, 1, 88));
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let shader_id = database.import_asset_path(&shader_path).unwrap();
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
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
    let material_id = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .id;

    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    assert_eq!(
        fs::read(config.imported_root.join(skeleton_path.path())).unwrap(),
        expected_skeleton
    );
    assert_eq!(
        fs::read(config.imported_root.join(animation_path.path())).unwrap(),
        expected_animation
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database
            .registry()
            .metadata_by_path(&animation_path)
            .unwrap()
            .dependencies,
        vec![skeleton_id]
    );
    assert_eq!(
        database
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![shader_id, texture_id, mesh_id]
    );

    let model_id_again = database.import_asset_path(&model_path).unwrap();
    assert_eq!(model_id_again, model_id);
    assert_eq!(
        database
            .registry()
            .metadata_by_path(&animation_path)
            .unwrap()
            .dependencies,
        vec![skeleton_id]
    );
    assert_eq!(
        database
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![shader_id, texture_id, mesh_id]
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![
            shader_id,
            texture_id,
            mesh_id,
            skeleton_id,
            animation_id,
            material_id
        ]
    );

    database.save_registry().unwrap();
    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&animation_path)
            .unwrap()
            .dependencies,
        vec![skeleton_id]
    );
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![shader_id, texture_id, mesh_id]
    );

    for id in [
        shader_id,
        texture_id,
        skeleton_id,
        animation_id,
        material_id,
    ] {
        database.cook_asset(id, TargetPlatform::Windows).unwrap();
    }
    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let expected_cooked_mesh = simple_binary_mesh_bytes();
    assert_eq!(mesh_output.bytes, expected_cooked_mesh);
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "block_model",
            vec![
                mesh_id,
                skeleton_id,
                animation_id,
                material_id,
                shader_id,
                texture_id,
            ],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(animation_id),
        Some([skeleton_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([shader_id, texture_id, mesh_id].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), expected_cooked_mesh);
    assert_eq!(
        reader.read_path(&animation_path).unwrap(),
        expected_animation
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
}

#[test]
fn database_model_importer_accepts_manifest_structural_comments() {
    let config = database_config("builtin_model_manifest_structural_comments");
    let model_path = AssetPath::parse("models/commented_manifest.model");
    let mesh_path = AssetPath::parse("models/commented_manifest.Body.mesh");
    let material_path = AssetPath::parse("models/commented_manifest.HeroMaterial.material");
    let model_source = b"NGA_MODEL_V1 # manifest header comment
# top-level comment
mesh=Body # generated mesh label comment
--- # mesh payload follows
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end # mesh block end
material=HeroMaterial # generated material label comment
depends=mesh:Body # local generated dependency comment
--- # material payload follows
name=hero
base_color=1,1,1,1
end # material block end
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();

    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(material_metadata.importer_version, 111);
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        mesh_bytes()
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        b"name=hero\nbase_color=1,1,1,1\n".to_vec()
    );
    assert_eq!(material_metadata.dependencies, vec![mesh_metadata.id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_metadata.id, material_metadata.id]
    );
}

#[test]
fn database_model_importer_unquotes_manifest_generated_labels() {
    let config = database_config("builtin_model_manifest_quoted_labels");
    let model_path = AssetPath::parse("models/quoted_manifest.model");
    let mesh_path = AssetPath::parse("models/quoted_manifest.Body__Main.mesh");
    let hero_material_path = AssetPath::parse("models/quoted_manifest.Hero__Material.material");
    let detail_material_path = AssetPath::parse("models/quoted_manifest.Detail_Material.material");
    let skeleton_path = AssetPath::parse("models/quoted_manifest.Rig__Main.skeleton");
    let animation_path = AssetPath::parse("models/quoted_manifest.Walk__Anim.animation");
    let model_source = b"NGA_MODEL_V1
mesh=\"Body #Main\" # quoted label keeps hash
materials=\"Hero #Material\",\"Detail,Material\" # quoted list keeps comma
---
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
material=\"Hero #Material\"
name=hero
base_color=1,1,1,1
end
material=\"Detail,Material\"|name=detail;base_color=0.5,0.5,0.5,1
skeleton=\"Rig #Main\"
NGA_SKELETON_V1
bone=Root
end
animation=\"Walk #Anim\"
depends=skeleton:\"Rig #Main\"
NGA_ANIMATION_V1
duration=1
ticks_per_second=24
track=bone:Root
translation=0:0,0,0
end
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let hero_metadata = database
        .registry()
        .metadata_by_path(&hero_material_path)
        .unwrap();
    let detail_metadata = database
        .registry()
        .metadata_by_path(&detail_material_path)
        .unwrap();
    let skeleton_metadata = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap();
    let animation_metadata = database
        .registry()
        .metadata_by_path(&animation_path)
        .unwrap();

    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(hero_metadata.importer_version, 111);
    assert_eq!(detail_metadata.importer_version, 111);
    assert_eq!(skeleton_metadata.importer_version, 111);
    assert_eq!(animation_metadata.importer_version, 111);
    assert_eq!(mesh_metadata.labels, vec!["Body #Main"]);
    assert_eq!(hero_metadata.labels, vec!["Hero #Material"]);
    assert_eq!(detail_metadata.labels, vec!["Detail,Material"]);
    assert_eq!(skeleton_metadata.labels, vec!["Rig #Main"]);
    assert_eq!(animation_metadata.labels, vec!["Walk #Anim"]);
    assert_eq!(
        mesh_metadata.dependencies,
        vec![hero_metadata.id, detail_metadata.id]
    );
    assert!(hero_metadata.dependencies.is_empty());
    assert!(detail_metadata.dependencies.is_empty());
    assert_eq!(animation_metadata.dependencies, vec![skeleton_metadata.id]);
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        mesh_bytes()
    );
    assert_eq!(
        fs::read(config.imported_root.join(hero_material_path.path())).unwrap(),
        b"name=hero\nbase_color=1,1,1,1\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(detail_material_path.path())).unwrap(),
        b"name=detail\nbase_color=0.5,0.5,0.5,1\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(skeleton_path.path())).unwrap(),
        b"NGA_SKELETON_V1\nbone=Root\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(animation_path.path())).unwrap(),
        b"NGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=bone:Root\ntranslation=0:0,0,0\n"
            .to_vec()
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![
            mesh_metadata.id,
            hero_metadata.id,
            detail_metadata.id,
            skeleton_metadata.id,
            animation_metadata.id
        ]
    );
}

#[test]
fn database_model_importer_reports_invalid_manifest_quoted_labels() {
    let label_config = database_config("builtin_model_manifest_unterminated_label");
    let model_path = AssetPath::parse("models/bad_quoted_label.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=\"Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(label_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_quoted_label.model")
                && message.contains("model mesh label has unterminated \" quote on line 2")
    ));

    let list_config = database_config("builtin_model_manifest_unterminated_label_list");
    let model_path = AssetPath::parse("models/bad_quoted_label_list.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nmaterials=\"Hero\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(list_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_quoted_label_list.model")
                && message.contains("model dependency list has unterminated \" quote on line 3")
    ));
}

#[test]
fn database_model_importer_accepts_manifest_binding_aliases() {
    let config = database_config("builtin_model_manifest_binding_aliases");
    let model_path = AssetPath::parse("models/alias_bindings.model");
    let body_path = AssetPath::parse("models/alias_bindings.Body.mesh");
    let lod_path = AssetPath::parse("models/alias_bindings.Body_LOD0.mesh");
    let hero_material_path = AssetPath::parse("models/alias_bindings.HeroMaterial.material");
    let overlay_material_path = AssetPath::parse("models/alias_bindings.Overlay.material");
    let collision_path = AssetPath::parse("models/alias_bindings.Collision.physics");
    let proxy_path = AssetPath::parse("models/alias_bindings.Proxy.physics");
    let model_source = b"NGA_MODEL_V1
mesh=Body
material_slots=HeroMaterial
collision_mesh=Collision
lod_mesh=Body.LOD0
---
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
mesh=Body.LOD0
v 0 0 0
v 0.5 0 0
v 0 0.5 0
i 0 1 2
end
material=HeroMaterial
name=hero
base_color=1,1,1,1
end
physics_mesh=Collision
NGA_PHYSICS_MESH_V1
kind=trimesh
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
material=Overlay
render_mesh=Body.LOD0
name=overlay
base_color=0.25,0.5,0.75,1
end
physics_mesh=Proxy
source_mesh=Body.LOD0
NGA_PHYSICS_MESH_V1
kind=trimesh
v 0 0 0
v 0.5 0 0
v 0 0.5 0
i 0 1 2
end
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let body_metadata = database.registry().metadata_by_path(&body_path).unwrap();
    let lod_metadata = database.registry().metadata_by_path(&lod_path).unwrap();
    let hero_metadata = database
        .registry()
        .metadata_by_path(&hero_material_path)
        .unwrap();
    let overlay_metadata = database
        .registry()
        .metadata_by_path(&overlay_material_path)
        .unwrap();
    let collision_metadata = database
        .registry()
        .metadata_by_path(&collision_path)
        .unwrap();
    let proxy_metadata = database.registry().metadata_by_path(&proxy_path).unwrap();

    assert_eq!(body_metadata.importer_version, 111);
    assert_eq!(hero_metadata.importer_version, 111);
    assert_eq!(collision_metadata.importer_version, 111);
    assert_eq!(
        body_metadata.dependencies,
        vec![hero_metadata.id, collision_metadata.id, lod_metadata.id]
    );
    assert_eq!(overlay_metadata.dependencies, vec![lod_metadata.id]);
    assert_eq!(proxy_metadata.dependencies, vec![lod_metadata.id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![
            body_metadata.id,
            lod_metadata.id,
            hero_metadata.id,
            collision_metadata.id,
            overlay_metadata.id,
            proxy_metadata.id
        ]
    );
    assert_eq!(
        fs::read(config.imported_root.join(body_path.path())).unwrap(),
        mesh_bytes()
    );
    assert_eq!(
        fs::read(config.imported_root.join(lod_path.path())).unwrap(),
        b"v 0 0 0\nv 0.5 0 0\nv 0 0.5 0\ni 0 1 2\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(hero_material_path.path())).unwrap(),
        b"name=hero\nbase_color=1,1,1,1\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(overlay_material_path.path())).unwrap(),
        b"name=overlay\nbase_color=0.25,0.5,0.75,1\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(collision_path.path())).unwrap(),
        physics_mesh_bytes()
    );
    assert_eq!(
        fs::read(config.imported_root.join(proxy_path.path())).unwrap(),
        b"NGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 0.5 0 0\nv 0 0.5 0\ni 0 1 2\n".to_vec()
    );
}

#[test]
fn database_model_importer_accepts_case_insensitive_manifest_structure_keys() {
    let config = database_config("builtin_model_manifest_case_insensitive_keys");
    let model_path = AssetPath::parse("models/case_manifest.model");
    let mesh_path = AssetPath::parse("models/case_manifest.Body.mesh");
    let material_path = AssetPath::parse("models/case_manifest.Hero.material");
    let overlay_material_path = AssetPath::parse("models/case_manifest.Overlay.material");
    let skeleton_path = AssetPath::parse("models/case_manifest.Rig.skeleton");
    let animation_path = AssetPath::parse("models/case_manifest.Walk.animation");
    let physics_path = AssetPath::parse("models/case_manifest.Collision.physics");
    let proxy_physics_path = AssetPath::parse("models/case_manifest.Proxy.physics");
    let model_source = b"NGA_MODEL_V1
Mesh=Body
Material_Slots=Hero
Collision_Mesh=Collision
Skin=Rig
Skin_Root=Root
---
v 0 0 0
v 1 0 0
v 0 1 0
j 0 0 0 0
j 0 0 0 0
j 0 0 0 0
w 1 0 0 0
w 1 0 0 0
w 1 0 0 0
i 0 1 2
end
MATERIAL=Hero
name=hero
base_color=1,1,1,1
end
Skeleton=Rig
NGA_SKELETON_V1
bone=Root
end
Animation=Walk
Depends=Skeleton:Rig
NGA_ANIMATION_V1
duration=1
ticks_per_second=24
track=bone:Root
translation=0:0,0,0
end
PHYSICS_MESH=Collision
NGA_PHYSICS_MESH_V1
kind=trimesh
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
Material=Overlay
Target_Render_Mesh=Body
name=overlay
base_color=0.25,0.5,0.75,1
end
Physics_Mesh=Proxy
Source_Mesh=Body
NGA_PHYSICS_MESH_V1
kind=trimesh
v 0 0 0
v 0.5 0 0
v 0 0.5 0
i 0 1 2
end
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let overlay_metadata = database
        .registry()
        .metadata_by_path(&overlay_material_path)
        .unwrap();
    let skeleton_metadata = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap();
    let animation_metadata = database
        .registry()
        .metadata_by_path(&animation_path)
        .unwrap();
    let physics_metadata = database.registry().metadata_by_path(&physics_path).unwrap();
    let proxy_metadata = database
        .registry()
        .metadata_by_path(&proxy_physics_path)
        .unwrap();

    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(material_metadata.importer_version, 111);
    assert_eq!(overlay_metadata.importer_version, 111);
    assert_eq!(skeleton_metadata.importer_version, 111);
    assert_eq!(animation_metadata.importer_version, 111);
    assert_eq!(physics_metadata.importer_version, 111);
    assert_eq!(proxy_metadata.importer_version, 111);
    assert_eq!(
        mesh_metadata.dependencies,
        vec![
            material_metadata.id,
            physics_metadata.id,
            skeleton_metadata.id
        ]
    );
    assert!(material_metadata.dependencies.is_empty());
    assert_eq!(overlay_metadata.dependencies, vec![mesh_metadata.id]);
    assert_eq!(animation_metadata.dependencies, vec![skeleton_metadata.id]);
    assert!(physics_metadata.dependencies.is_empty());
    assert_eq!(proxy_metadata.dependencies, vec![mesh_metadata.id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![
            mesh_metadata.id,
            material_metadata.id,
            skeleton_metadata.id,
            animation_metadata.id,
            physics_metadata.id,
            overlay_metadata.id,
            proxy_metadata.id
        ]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 0 0 0\nj 0 0 0 0\nj 0 0 0 0\nw 1 0 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\n"
            .to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        b"name=hero\nbase_color=1,1,1,1\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(animation_path.path())).unwrap(),
        b"NGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=bone:Root\ntranslation=0:0,0,0\n"
            .to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(overlay_material_path.path())).unwrap(),
        b"name=overlay\nbase_color=0.25,0.5,0.75,1\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(proxy_physics_path.path())).unwrap(),
        b"NGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 0.5 0 0\nv 0 0.5 0\ni 0 1 2\n".to_vec()
    );
}

#[test]
fn database_model_importer_accepts_manifest_generated_kind_aliases() {
    let config = database_config("builtin_model_manifest_kind_aliases");
    let model_path = AssetPath::parse("models/kind_aliases.model");
    let mesh_path = AssetPath::parse("models/kind_aliases.Body.mesh");
    let material_path = AssetPath::parse("models/kind_aliases.Hero.material");
    let skeleton_path = AssetPath::parse("models/kind_aliases.Rig.skeleton");
    let animation_path = AssetPath::parse("models/kind_aliases.Walk.animation");
    let physics_path = AssetPath::parse("models/kind_aliases.Collision.physics");
    let model_source = b"NGA_MODEL_V1
Geometry=Body
material=Hero
---
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
mat=Hero
name=hero
base_color=1,1,1,1
end
skel=Rig
NGA_SKELETON_V1
bone=Root
end
anim=Walk
depends=skel:Rig
NGA_ANIMATION_V1
duration=1
ticks_per_second=24
track=bone:Root
translation=0:0,0,0
end
collision_mesh=Collision
depends=geometry:Body
NGA_PHYSICS_MESH_V1
kind=trimesh
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let skeleton_metadata = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap();
    let animation_metadata = database
        .registry()
        .metadata_by_path(&animation_path)
        .unwrap();
    let physics_metadata = database.registry().metadata_by_path(&physics_path).unwrap();

    assert_eq!(mesh_metadata.asset_type, Mesh::TYPE_ID);
    assert_eq!(material_metadata.asset_type, Material::TYPE_ID);
    assert_eq!(skeleton_metadata.asset_type, Skeleton::TYPE_ID);
    assert_eq!(animation_metadata.asset_type, AnimationClip::TYPE_ID);
    assert_eq!(physics_metadata.asset_type, PhysicsMesh::TYPE_ID);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(material_metadata.importer_version, 111);
    assert_eq!(skeleton_metadata.importer_version, 111);
    assert_eq!(animation_metadata.importer_version, 111);
    assert_eq!(physics_metadata.importer_version, 111);
    assert_eq!(mesh_metadata.dependencies, vec![material_metadata.id]);
    assert!(material_metadata.dependencies.is_empty());
    assert!(skeleton_metadata.dependencies.is_empty());
    assert_eq!(animation_metadata.dependencies, vec![skeleton_metadata.id]);
    assert_eq!(physics_metadata.dependencies, vec![mesh_metadata.id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![
            mesh_metadata.id,
            material_metadata.id,
            skeleton_metadata.id,
            animation_metadata.id,
            physics_metadata.id
        ]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        mesh_bytes()
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        b"name=hero\nbase_color=1,1,1,1\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(skeleton_path.path())).unwrap(),
        b"NGA_SKELETON_V1\nbone=Root\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(animation_path.path())).unwrap(),
        b"NGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=bone:Root\ntranslation=0:0,0,0\n"
            .to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(physics_path.path())).unwrap(),
        physics_mesh_bytes()
    );
}

#[test]
fn database_model_importer_accepts_manifest_skeleton_metadata_aliases() {
    let config = database_config("builtin_model_manifest_skeleton_metadata_aliases");
    let model_path = AssetPath::parse("models/skeleton_aliases.model");
    let mesh_path = AssetPath::parse("models/skeleton_aliases.Body.mesh");
    let skeleton_path = AssetPath::parse("models/skeleton_aliases.Rig.skeleton");
    let animation_path = AssetPath::parse("models/skeleton_aliases.Walk.animation");
    let model_source = b"NGA_MODEL_V1
mesh=Body
rig=Rig
root_joint=Root
joint_limit=1
influence_limit=1
---
v 0 0 0
v 1 0 0
v 0 1 0
j 0 0 0 0
j 0 0 0 0
j 0 0 0 0
w 1 0 0 0
w 1 0 0 0
w 1 0 0 0
i 0 1 2
end
skeleton=Rig
NGA_SKELETON_V1
bone=Root
end
animation=Walk
target_rig=Rig
NGA_ANIMATION_V1
duration=1
ticks_per_second=24
track=bone:Root
translation=0:0,0,0
end
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let skeleton_metadata = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap();
    let animation_metadata = database
        .registry()
        .metadata_by_path(&animation_path)
        .unwrap();

    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(skeleton_metadata.importer_version, 111);
    assert_eq!(animation_metadata.importer_version, 111);
    assert_eq!(mesh_metadata.dependencies, vec![skeleton_metadata.id]);
    assert_eq!(animation_metadata.dependencies, vec![skeleton_metadata.id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![
            mesh_metadata.id,
            skeleton_metadata.id,
            animation_metadata.id
        ]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 0 0 0\nj 0 0 0 0\nj 0 0 0 0\nw 1 0 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\n"
            .to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(skeleton_path.path())).unwrap(),
        b"NGA_SKELETON_V1\nbone=Root\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(animation_path.path())).unwrap(),
        b"NGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=bone:Root\ntranslation=0:0,0,0\n"
            .to_vec()
    );
}

#[test]
fn database_model_importer_accepts_manifest_dependency_metadata_aliases() {
    let config = database_config("builtin_model_manifest_dependency_metadata_aliases");
    let model_path = AssetPath::parse("models/dependency_aliases.model");
    let body_path = AssetPath::parse("models/dependency_aliases.Body.mesh");
    let debug_path = AssetPath::parse("models/dependency_aliases.Debug.mesh");
    let hero_material_path = AssetPath::parse("models/dependency_aliases.Hero.material");
    let overlay_material_path = AssetPath::parse("models/dependency_aliases.Overlay.material");
    let skeleton_path = AssetPath::parse("models/dependency_aliases.Rig.skeleton");
    let animation_path = AssetPath::parse("models/dependency_aliases.Walk.animation");
    let physics_path = AssetPath::parse("models/dependency_aliases.Collision.physics");
    let model_source = b"NGA_MODEL_V1
mesh=Body
dependencies=mat:Hero
---
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
material=Hero
name=hero
base_color=1,1,1,1
end
skeleton=Rig
NGA_SKELETON_V1
bone=Root
end
animation=Walk
requires=rig:Rig
NGA_ANIMATION_V1
duration=1
ticks_per_second=24
track=bone:Root
translation=0:0,0,0
end
physics_mesh=Collision
refs=geometry:Body
NGA_PHYSICS_MESH_V1
kind=trimesh
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
material=Overlay
references=physics:Collision
name=overlay
base_color=0.25,0.5,0.75,1
end
mesh=Debug
require=render_mesh:Body
v 0 0 0
v 0 1 0
v 1 0 0
i 0 1 2
end
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let body_metadata = database.registry().metadata_by_path(&body_path).unwrap();
    let debug_metadata = database.registry().metadata_by_path(&debug_path).unwrap();
    let hero_metadata = database
        .registry()
        .metadata_by_path(&hero_material_path)
        .unwrap();
    let overlay_metadata = database
        .registry()
        .metadata_by_path(&overlay_material_path)
        .unwrap();
    let skeleton_metadata = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap();
    let animation_metadata = database
        .registry()
        .metadata_by_path(&animation_path)
        .unwrap();
    let physics_metadata = database.registry().metadata_by_path(&physics_path).unwrap();

    assert_eq!(body_metadata.importer_version, 111);
    assert_eq!(debug_metadata.importer_version, 111);
    assert_eq!(hero_metadata.importer_version, 111);
    assert_eq!(overlay_metadata.importer_version, 111);
    assert_eq!(skeleton_metadata.importer_version, 111);
    assert_eq!(animation_metadata.importer_version, 111);
    assert_eq!(physics_metadata.importer_version, 111);
    assert_eq!(body_metadata.dependencies, vec![hero_metadata.id]);
    assert!(hero_metadata.dependencies.is_empty());
    assert!(skeleton_metadata.dependencies.is_empty());
    assert_eq!(animation_metadata.dependencies, vec![skeleton_metadata.id]);
    assert_eq!(physics_metadata.dependencies, vec![body_metadata.id]);
    assert_eq!(overlay_metadata.dependencies, vec![physics_metadata.id]);
    assert_eq!(debug_metadata.dependencies, vec![body_metadata.id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![
            body_metadata.id,
            hero_metadata.id,
            skeleton_metadata.id,
            animation_metadata.id,
            physics_metadata.id,
            overlay_metadata.id,
            debug_metadata.id
        ]
    );
    assert_eq!(
        fs::read(config.imported_root.join(body_path.path())).unwrap(),
        mesh_bytes()
    );
    assert_eq!(
        fs::read(config.imported_root.join(debug_path.path())).unwrap(),
        b"v 0 0 0\nv 0 1 0\nv 1 0 0\ni 0 1 2\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(hero_material_path.path())).unwrap(),
        b"name=hero\nbase_color=1,1,1,1\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(overlay_material_path.path())).unwrap(),
        b"name=overlay\nbase_color=0.25,0.5,0.75,1\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(skeleton_path.path())).unwrap(),
        b"NGA_SKELETON_V1\nbone=Root\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(animation_path.path())).unwrap(),
        b"NGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=bone:Root\ntranslation=0:0,0,0\n"
            .to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(physics_path.path())).unwrap(),
        physics_mesh_bytes()
    );
}

#[test]
fn database_model_importer_accepts_separator_insensitive_manifest_structure_keys() {
    let config = database_config("builtin_model_manifest_separator_insensitive_keys");
    let model_path = AssetPath::parse("models/separator_manifest.model");
    let body_path = AssetPath::parse("models/separator_manifest.Body.mesh");
    let lod_path = AssetPath::parse("models/separator_manifest.BodyLOD.mesh");
    let hero_material_path = AssetPath::parse("models/separator_manifest.Hero.material");
    let overlay_material_path = AssetPath::parse("models/separator_manifest.Overlay.material");
    let skeleton_path = AssetPath::parse("models/separator_manifest.Rig.skeleton");
    let animation_path = AssetPath::parse("models/separator_manifest.Walk.animation");
    let collision_path = AssetPath::parse("models/separator_manifest.Collision.physics");
    let proxy_path = AssetPath::parse("models/separator_manifest.Proxy.physics");
    let model_source = b"NGA_MODEL_V1
Render-Mesh=Body
material slot=Hero
lod-level=BodyLOD
---
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
mesh=BodyLOD
v 0 0 0
v 0.5 0 0
v 0 0.5 0
i 0 1 2
end
Mat=Hero
name=hero
base_color=1,1,1,1
end
Skeleton=Rig
NGA_SKELETON_V1
bone=Root
end
Animation Clip=Walk
target rig=Rig
NGA_ANIMATION_V1
duration=1
ticks_per_second=24
track=bone:Root
translation=0:0,0,0
end
Physics-Mesh=Collision
NGA_PHYSICS_MESH_V1
kind=trimesh
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
Physics Mesh=Proxy
source mesh=BodyLOD
NGA_PHYSICS_MESH_V1
kind=trimesh
v 0 0 0
v 0.5 0 0
v 0 0.5 0
i 0 1 2
end
Material=Overlay
depends=physics-mesh:Collision
target render mesh=Body
name=overlay
base_color=0.25,0.5,0.75,1
end
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let body_metadata = database.registry().metadata_by_path(&body_path).unwrap();
    let lod_metadata = database.registry().metadata_by_path(&lod_path).unwrap();
    let hero_metadata = database
        .registry()
        .metadata_by_path(&hero_material_path)
        .unwrap();
    let overlay_metadata = database
        .registry()
        .metadata_by_path(&overlay_material_path)
        .unwrap();
    let skeleton_metadata = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap();
    let animation_metadata = database
        .registry()
        .metadata_by_path(&animation_path)
        .unwrap();
    let collision_metadata = database
        .registry()
        .metadata_by_path(&collision_path)
        .unwrap();
    let proxy_metadata = database.registry().metadata_by_path(&proxy_path).unwrap();

    assert_eq!(body_metadata.asset_type, Mesh::TYPE_ID);
    assert_eq!(lod_metadata.asset_type, Mesh::TYPE_ID);
    assert_eq!(hero_metadata.asset_type, Material::TYPE_ID);
    assert_eq!(overlay_metadata.asset_type, Material::TYPE_ID);
    assert_eq!(skeleton_metadata.asset_type, Skeleton::TYPE_ID);
    assert_eq!(animation_metadata.asset_type, AnimationClip::TYPE_ID);
    assert_eq!(collision_metadata.asset_type, PhysicsMesh::TYPE_ID);
    assert_eq!(proxy_metadata.asset_type, PhysicsMesh::TYPE_ID);
    assert_eq!(body_metadata.importer_version, 111);
    assert_eq!(lod_metadata.importer_version, 111);
    assert_eq!(hero_metadata.importer_version, 111);
    assert_eq!(overlay_metadata.importer_version, 111);
    assert_eq!(skeleton_metadata.importer_version, 111);
    assert_eq!(animation_metadata.importer_version, 111);
    assert_eq!(collision_metadata.importer_version, 111);
    assert_eq!(proxy_metadata.importer_version, 111);
    assert_eq!(
        body_metadata.dependencies,
        vec![hero_metadata.id, lod_metadata.id]
    );
    assert!(lod_metadata.dependencies.is_empty());
    assert!(hero_metadata.dependencies.is_empty());
    assert!(skeleton_metadata.dependencies.is_empty());
    assert_eq!(animation_metadata.dependencies, vec![skeleton_metadata.id]);
    assert!(collision_metadata.dependencies.is_empty());
    assert_eq!(proxy_metadata.dependencies, vec![lod_metadata.id]);
    assert_eq!(
        overlay_metadata.dependencies,
        vec![collision_metadata.id, body_metadata.id]
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![
            body_metadata.id,
            lod_metadata.id,
            hero_metadata.id,
            skeleton_metadata.id,
            animation_metadata.id,
            collision_metadata.id,
            proxy_metadata.id,
            overlay_metadata.id
        ]
    );
    assert_eq!(
        fs::read(config.imported_root.join(body_path.path())).unwrap(),
        mesh_bytes()
    );
    assert_eq!(
        fs::read(config.imported_root.join(lod_path.path())).unwrap(),
        b"v 0 0 0\nv 0.5 0 0\nv 0 0.5 0\ni 0 1 2\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(hero_material_path.path())).unwrap(),
        b"name=hero\nbase_color=1,1,1,1\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(overlay_material_path.path())).unwrap(),
        b"name=overlay\nbase_color=0.25,0.5,0.75,1\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(skeleton_path.path())).unwrap(),
        b"NGA_SKELETON_V1\nbone=Root\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(animation_path.path())).unwrap(),
        b"NGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=bone:Root\ntranslation=0:0,0,0\n"
            .to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(collision_path.path())).unwrap(),
        physics_mesh_bytes()
    );
    assert_eq!(
        fs::read(config.imported_root.join(proxy_path.path())).unwrap(),
        b"NGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 0.5 0 0\nv 0 0.5 0\ni 0 1 2\n".to_vec()
    );
}

#[test]
fn database_model_importer_applies_model_import_settings() {
    let config = database_config("builtin_model_import_settings");
    let model_path = AssetPath::parse("models/settings.obj");
    let mesh_path = AssetPath::parse("models/settings.Mesh0.mesh");
    let material_path = AssetPath::parse("models/settings.Material_Red.material");
    let obj_source = b"usemtl Red\nv 1 0 0\nv 0 1 0\nv 0 0 1\nvt 0 0\nvt 1 0\nvt 0 1\nvn 0 0 1\nf 1/1/1 2/2/1 3/3/1\n".to_vec();
    let expected_mesh =
        b"v 2 0 0\nv 0 2 0\nv 0 0 2\nn 0 0 1\nn 0 0 1\nn 0 0 1\nuv 0 0\nuv 1 0\nuv 0 1\ni 0 1 2\n"
            .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), obj_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let mut settings = ImporterSettings::default();
    settings.set("scale", "2");
    settings.set("generate_tangents", "false");
    settings.set("import_materials", "false");
    let model_id = database
        .import_asset_path_with_settings(&model_path, &settings)
        .unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();

    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    assert!(database
        .registry()
        .metadata_by_path(&material_path)
        .is_none());
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_metadata.id]
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().importer_settings,
        vec![
            ("generate_tangents".to_owned(), "false".to_owned()),
            ("import_materials".to_owned(), "false".to_owned()),
            ("scale".to_owned(), "2".to_owned()),
        ]
    );
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .settings_hash
        .is_some());
}

#[test]
fn database_model_importer_can_filter_generated_mesh_subresources() {
    let config = database_config("builtin_model_filter_mesh_subresources");
    let model_path = AssetPath::parse("models/filtered_mesh.model");
    let mesh_path = AssetPath::parse("models/filtered_mesh.Body.mesh");
    let material_path = AssetPath::parse("models/filtered_mesh.RedMaterial.material");
    let model_source = b"NGA_MODEL_V1\nmesh=Body\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=RedMaterial\ntarget_mesh=Body\nname=red\nbase_color=0.25,0.5,0.75,1\nend\n".to_vec();
    let expected_material = b"name=red\nbase_color=0.25,0.5,0.75,1\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let mut settings = ImporterSettings::default();
    settings.set("import_meshes", "false");
    let model_id = database
        .import_asset_path_with_settings(&model_path, &settings)
        .unwrap();

    assert!(database.registry().metadata_by_path(&mesh_path).is_none());
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    assert_eq!(material_metadata.dependencies, vec![]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![material_metadata.id]
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().importer_settings,
        vec![("import_meshes".to_owned(), "false".to_owned())]
    );
}

#[test]
fn database_model_importer_reports_invalid_model_import_settings() {
    for (case, key, value, expected_message) in [
        (
            "builtin_model_import_settings_bad_bool",
            "generate_tangents",
            "maybe",
            "invalid model import setting `generate_tangents` value `maybe`; expected true or false",
        ),
        (
            "builtin_model_import_settings_bad_physics_mesh_bool",
            "import_physics_meshes",
            "sometimes",
            "invalid model import setting `import_physics_meshes` value `sometimes`; expected true or false",
        ),
        (
            "builtin_model_import_settings_bad_scale",
            "scale",
            "0",
            "invalid model import setting `scale` value `0`; expected a finite positive scale",
        ),
    ] {
        let config = database_config(case);
        let model_path = AssetPath::parse("models/settings_error.model");
        let mut io = MemoryAssetIo::new();
        io.insert(
            model_path.path(),
            b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n".to_vec(),
        );
        let mut database = AssetDatabase::new(config);
        database.set_io(io);
        database.register_builtin_importers();

        let mut settings = ImporterSettings::default();
        settings.set(key, value);
        let error = database
            .import_asset_path_with_settings(&model_path, &settings)
            .unwrap_err();
        assert!(
            matches!(error, AssetError::Import { ref message } if message.contains("ModelImporter")
                && message.contains("models/settings_error.model")
                && message.contains(expected_message)),
            "{error:?}"
        );
    }
}

#[test]
fn database_model_importer_optimizes_model_meshes_when_enabled() {
    let config = database_config("builtin_model_optimize_meshes");
    let model_path = AssetPath::parse("models/optimized.model");
    let mesh_path = AssetPath::parse("models/optimized.Body.mesh");
    let model_source = b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 0 0\nv 0 1 0\nv 0 0 0\nv 2 0 0\nv 9 9 9\nn 0 0 1\nn 0 0 1\nn 0 0 1\nn 0 0 1\nn 0 0 1\nn 0 0 1\nn 0 1 0\nn 1 0 0\nuv 0 0\nuv 1 0\nuv 0 1\nuv 1 0\nuv 0 1\nuv 0 0\nuv 2 0\nuv 1 1\ni 0 1 2\ni 3 4 5\ni 0 1 1\ni 0 1 6\nend\n".to_vec();
    let expected_mesh =
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nn 0 0 1\nn 0 0 1\nn 0 0 1\nuv 0 0\nuv 1 0\nuv 0 1\ni 0 1 2\ni 1 2 0\n"
            .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let mut settings = ImporterSettings::default();
    settings.set("optimize_meshes", "true");
    let model_id = database
        .import_asset_path_with_settings(&model_path, &settings)
        .unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();

    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_metadata.id]
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().importer_settings,
        vec![("optimize_meshes".to_owned(), "true".to_owned())]
    );
}

#[test]
fn database_model_importer_rejects_all_degenerate_optimized_triangles() {
    let config = database_config("builtin_model_optimize_meshes_all_degenerate");
    let model_path = AssetPath::parse("models/all_degenerate.model");
    let model_source =
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 2 0 0\ni 0 1 1\ni 0 1 2\nend\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let mut settings = ImporterSettings::default();
    settings.set("optimize_meshes", "true");
    let error = database
        .import_asset_path_with_settings(&model_path, &settings)
        .unwrap_err();
    assert!(
        matches!(error, AssetError::Import { ref message } if message.contains("ModelImporter")
            && message.contains("models/all_degenerate.model")
            && message.contains("model mesh optimization removed all triangles as degenerate")),
        "{error:?}"
    );
}

#[test]
fn database_model_importer_generates_lod_meshes_when_enabled() {
    let config = database_config("builtin_model_generate_lods");
    let model_path = AssetPath::parse("models/lods.model");
    let mesh_path = AssetPath::parse("models/lods.Body.mesh");
    let lod_mesh_path = AssetPath::parse("models/lods.Body_LOD1.mesh");
    let material_path = AssetPath::parse("models/lods.HeroMaterial.material");
    let model_source = b"NGA_MODEL_V1\nmesh=Body\nmaterial=HeroMaterial\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 1 0\nv 2 0 0\nn 0 0 1\nn 0 0 1\nn 0 0 1\nn 0 0 1\nn 0 1 0\nuv 0 0\nuv 1 0\nuv 0 1\nuv 1 1\nuv 2 0\ni 0 1 2\ni 1 3 2\ni 0 1 4\ni 0 2 3\nend\nmaterial=HeroMaterial\nname=hero\nbase_color=1,1,1,1\nend\n".to_vec();
    let expected_lod_mesh =
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 1 0\nn 0 0 1\nn 0 0 1\nn 0 0 1\nn 0 0 1\nuv 0 0\nuv 1 0\nuv 0 1\nuv 1 1\ni 0 1 2\ni 0 2 3\n"
            .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let mut settings = ImporterSettings::default();
    settings.set("generate_lods", "true");
    let model_id = database
        .import_asset_path_with_settings(&model_path, &settings)
        .unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let lod_metadata = database
        .registry()
        .metadata_by_path(&lod_mesh_path)
        .unwrap();
    let material_id = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .id;

    assert_eq!(
        fs::read(config.imported_root.join(lod_mesh_path.path())).unwrap(),
        expected_lod_mesh
    );
    assert_eq!(lod_metadata.dependencies, vec![material_id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, lod_metadata.id, material_id]
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().importer_settings,
        vec![("generate_lods".to_owned(), "true".to_owned())]
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_records_mesh_lod_binding_metadata() {
    let config = database_config("builtin_model_mesh_lod_binding");
    let model_path = AssetPath::parse("models/lod_binding.model");
    let mesh_path = AssetPath::parse("models/lod_binding.Body.mesh");
    let lod0_path = AssetPath::parse("models/lod_binding.Body_LOD0.mesh");
    let lod1_path = AssetPath::parse("models/lod_binding.Body_LOD1.mesh");
    let model_source = b"NGA_MODEL_V1\nmesh=Body\nlods=Body.LOD0,Body.LOD1\n---\nv 0 0 0\nv 2 0 0\nv 0 2 0\ni 0 1 2\nend\nmesh=Body.LOD0\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmesh=Body.LOD1\nv 0 0 0\nv 0.5 0 0\nv 0 0.5 0\ni 0 1 2\nend\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;
    let lod0_id = database.registry().metadata_by_path(&lod0_path).unwrap().id;
    let lod1_id = database.registry().metadata_by_path(&lod1_path).unwrap().id;

    assert_eq!(mesh_metadata.dependencies, vec![lod0_id, lod1_id]);
    assert_eq!(mesh_metadata.labels, vec!["Body"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, lod0_id, lod1_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&mesh_path)
            .unwrap()
            .dependencies,
        vec![lod0_id, lod1_id]
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(lod0_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(lod1_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "mesh_lod_binding",
            vec![mesh_id, lod0_id, lod1_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([lod0_id, lod1_id].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(lod0_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(lod1_id), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(mesh_id),
        &[lod0_id, lod1_id]
    );
    let server_mesh_scope = server.scoped_dependency_report(mesh_id).unwrap();
    assert!(server_mesh_scope.direct_dependencies.contains(&lod0_id));
    assert!(server_mesh_scope.direct_dependencies.contains(&lod1_id));
    assert!(server
        .dependency_report_text()
        .contains(&format!("edge|{}|{}", mesh_id.raw(), lod0_id.raw())));
    assert!(server
        .dependency_report_text()
        .contains(&format!("edge|{}|{}", mesh_id.raw(), lod1_id.raw())));
    assert!(server
        .dependency_report_json()
        .contains(&format!("\"{}\"", lod0_id.raw())));
    assert!(server
        .dependency_report_json()
        .contains(&format!("\"{}\"", lod1_id.raw())));
    assert!(server
        .dependency_report_html()
        .contains(&format!("<code>{}</code>", lod0_id.raw())));
    assert!(server
        .dependency_report_html()
        .contains(&format!("<code>{}</code>", lod1_id.raw())));
    let server_text_path = config.imported_root.join("mesh_lod_binding_server.txt");
    let server_dot_path = config.imported_root.join("mesh_lod_binding_server.dot");
    let server_json_path = config.imported_root.join("mesh_lod_binding_server.json");
    let server_html_path = config.imported_root.join("mesh_lod_binding_server.html");
    server
        .save_dependency_report_text(&server_text_path)
        .unwrap();
    server
        .save_dependency_report_dot(&server_dot_path)
        .unwrap();
    server
        .save_dependency_report_json(&server_json_path)
        .unwrap();
    server
        .save_dependency_report_html(&server_html_path)
        .unwrap();
    let server_text = fs::read_to_string(server_text_path).unwrap();
    assert!(server_text.contains(&format!("edge|{}|{}", mesh_id.raw(), lod0_id.raw())));
    assert!(server_text.contains(&format!("edge|{}|{}", mesh_id.raw(), lod1_id.raw())));
    let server_dot = fs::read_to_string(server_dot_path).unwrap();
    assert!(server_dot.contains(&format!("\"{}\" -> \"{}\";", mesh_id.raw(), lod0_id.raw())));
    assert!(server_dot.contains(&format!("\"{}\" -> \"{}\";", mesh_id.raw(), lod1_id.raw())));
    let server_json = fs::read_to_string(server_json_path).unwrap();
    assert!(server_json.contains(&format!("\"{}\"", lod0_id.raw())));
    assert!(server_json.contains(&format!("\"{}\"", lod1_id.raw())));
    let server_html = fs::read_to_string(server_html_path).unwrap();
    assert!(server_html.contains(&format!("<code>{}</code>", lod0_id.raw())));
    assert!(server_html.contains(&format!("<code>{}</code>", lod1_id.raw())));
    let mesh_scope = database.scoped_dependency_report(mesh_id).unwrap();
    assert!(mesh_scope.direct_dependencies.contains(&lod0_id));
    assert!(mesh_scope.direct_dependencies.contains(&lod1_id));
    assert!(database
        .dependency_report_text()
        .contains(&format!("edge|{}|{}", mesh_id.raw(), lod0_id.raw())));
    assert!(database
        .dependency_report_text()
        .contains(&format!("edge|{}|{}", mesh_id.raw(), lod1_id.raw())));
    assert!(database
        .dependency_report_json()
        .contains(&format!("\"{}\"", lod0_id.raw())));
    assert!(database
        .dependency_report_json()
        .contains(&format!("\"{}\"", lod1_id.raw())));
    assert!(database
        .dependency_report_html()
        .contains(&format!("<code>{}</code>", lod0_id.raw())));
    assert!(database
        .dependency_report_html()
        .contains(&format!("<code>{}</code>", lod1_id.raw())));

    let scoped_text_path = config.imported_root.join("mesh_lod_binding.txt");
    let scoped_dot_path = config.imported_root.join("mesh_lod_binding.dot");
    let scoped_json_path = config.imported_root.join("mesh_lod_binding.json");
    let scoped_html_path = config.imported_root.join("mesh_lod_binding.html");
    database
        .save_scoped_dependency_report_text(mesh_id, &scoped_text_path)
        .unwrap();
    database
        .save_scoped_dependency_report_dot(mesh_id, &scoped_dot_path)
        .unwrap();
    database
        .save_scoped_dependency_report_json(mesh_id, &scoped_json_path)
        .unwrap();
    database
        .save_scoped_dependency_report_html(mesh_id, &scoped_html_path)
        .unwrap();
    let scoped_text = fs::read_to_string(scoped_text_path).unwrap();
    assert!(scoped_text.contains(&format!("edge|{}|{}", mesh_id.raw(), lod0_id.raw())));
    assert!(scoped_text.contains(&format!("edge|{}|{}", mesh_id.raw(), lod1_id.raw())));
    let scoped_dot = fs::read_to_string(scoped_dot_path).unwrap();
    assert!(scoped_dot.contains(&format!("\"{}\" -> \"{}\";", mesh_id.raw(), lod0_id.raw())));
    assert!(scoped_dot.contains(&format!("\"{}\" -> \"{}\";", mesh_id.raw(), lod1_id.raw())));
    let scoped_json = fs::read_to_string(scoped_json_path).unwrap();
    assert!(scoped_json.contains(&format!("\"{}\"", lod0_id.raw())));
    assert!(scoped_json.contains(&format!("\"{}\"", lod1_id.raw())));
    let scoped_html = fs::read_to_string(scoped_html_path).unwrap();
    assert!(scoped_html.contains(&format!("<code>{}</code>", lod0_id.raw())));
    assert!(scoped_html.contains(&format!("<code>{}</code>", lod1_id.raw())));

    let unknown_config = database_config("builtin_model_unknown_mesh_lod_binding");
    let model_path = AssetPath::parse("models/unknown_lod_binding.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nlod=Missing\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(unknown_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();
    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/unknown_lod_binding.model")
                && message.contains("model mesh `Body`")
                && message.contains("references unknown LOD mesh `Missing`")
    ));

    let wrong_kind_config = database_config("builtin_model_mesh_lod_binding_wrong_kind");
    let model_path = AssetPath::parse("models/wrong_kind_lod_binding.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nlod=HeroMaterial\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\nname=hero\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(wrong_kind_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();
    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/wrong_kind_lod_binding.model")
                && message.contains("LOD binding `HeroMaterial`")
                && message.contains("references generated material `HeroMaterial` instead of a mesh")
    ));
}

#[test]
fn database_model_importer_reports_invalid_lod_generation_input() {
    let config = database_config("builtin_model_generate_lods_invalid_mesh");
    let model_path = AssetPath::parse("models/bad_lod.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\ni 0 1 3\ni 0 1 0\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let mut settings = ImporterSettings::default();
    settings.set("generate_lods", "true");
    let error = database
        .import_asset_path_with_settings(&model_path, &settings)
        .unwrap_err();
    assert!(
        matches!(error, AssetError::Import { ref message } if message.contains("ModelImporter")
            && message.contains("models/bad_lod.model")
            && message.contains("model mesh `Body` on line 2 LOD input is invalid")
            && message.contains("index 3 references missing vertex")),
        "{error:?}"
    );
}

#[test]
fn database_model_importer_rejects_all_degenerate_generated_lod_triangles() {
    let config = database_config("builtin_model_generate_lods_all_degenerate");
    let model_path = AssetPath::parse("models/all_degenerate_lod.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 2 0 0\ni 0 1 1\ni 0 1 2\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let mut settings = ImporterSettings::default();
    settings.set("generate_lods", "true");
    let error = database
        .import_asset_path_with_settings(&model_path, &settings)
        .unwrap_err();
    assert!(
        matches!(error, AssetError::Import { ref message } if message.contains("ModelImporter")
        && message.contains("models/all_degenerate_lod.model")
        && message.contains(
            "model mesh `Body` on line 2 LOD generation removed all triangles as degenerate"
        )),
        "{error:?}"
    );
}

#[test]
fn database_model_importer_records_typed_generated_dependencies() {
    let config = database_config("builtin_model_typed_generated_dependencies");
    let model_path = AssetPath::parse("models/typed_dependencies.model");
    let mesh_path = AssetPath::parse("models/typed_dependencies.Body.mesh");
    let skeleton_path = AssetPath::parse("models/typed_dependencies.Rig.skeleton");
    let animation_path = AssetPath::parse("models/typed_dependencies.Idle.animation");
    let material_path = AssetPath::parse("models/typed_dependencies.HeroMaterial.material");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\nanimation=Idle\ndepends=skeleton:Rig\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=bone:Root\ntranslation=0:0,0,0\nend\nmaterial=HeroMaterial\ndepends=mesh:Body,animation:Idle\nname=hero\nbase_color=1,1,1,1\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
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
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();

    assert_eq!(
        database
            .registry()
            .metadata_by_path(&animation_path)
            .unwrap()
            .dependencies,
        vec![skeleton_id]
    );
    assert_eq!(material_metadata.dependencies, vec![mesh_id, animation_id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, skeleton_id, animation_id, material_metadata.id]
    );

    let physics_config = database_config("builtin_model_typed_physics_mesh_dependency");
    let model_path = AssetPath::parse("models/physics_typed_dependencies.model");
    let physics_path = AssetPath::parse("models/physics_typed_dependencies.Collision.physics");
    let material_path = AssetPath::parse("models/physics_typed_dependencies.HeroMaterial.material");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nphysics_mesh=Collision\nNGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\ndepends=physics_mesh:Collision\nname=hero\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(physics_config);
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let physics_id = database
        .registry()
        .metadata_by_path(&physics_path)
        .unwrap()
        .id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();

    assert_eq!(material_metadata.dependencies, vec![physics_id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![physics_id, material_metadata.id]
    );

    let wrong_kind_config = database_config("builtin_model_typed_generated_dependency_wrong_kind");
    let model_path = AssetPath::parse("models/bad_typed_dependency.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\ndepends=skeleton:Body\nname=hero\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(wrong_kind_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_typed_dependency.model")
                && message.contains("model material `HeroMaterial`")
                && message.contains("dependency `Body` expected generated skeleton but found mesh `Body`")
    ));
}

#[test]
fn database_model_importer_rejects_generated_dependency_cycles() {
    let config = database_config("builtin_model_generated_dependency_cycle");
    let model_path = AssetPath::parse("models/dependency_cycle.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nmaterial=HeroMaterial\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\ntarget_mesh=Body\nname=hero\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/dependency_cycle.model")
                && message.contains("model generated dependency cycle detected")
                && message.contains("mesh `Body` on line 2")
                && message.contains("material `HeroMaterial` on line 9")
    ));
}

#[test]
fn database_model_importer_rejects_invalid_typed_generated_dependency_syntax() {
    let unknown_kind_config = database_config("builtin_model_typed_dependency_unknown_kind");
    let model_path = AssetPath::parse("models/unknown_typed_dependency.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\ndepends=light:Body\nname=hero\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(unknown_kind_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/unknown_typed_dependency.model")
                && message.contains("unknown model generated dependency kind `light`")
                && message.contains("expected mesh, material, skeleton, animation, or physics_mesh")
    ));

    let empty_label_config = database_config("builtin_model_typed_dependency_empty_label");
    let model_path = AssetPath::parse("models/empty_typed_dependency.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\ndepends=mesh:\nname=hero\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(empty_label_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/empty_typed_dependency.model")
                && message.contains("model dependency `mesh:`")
                && message.contains("must name a generated mesh label")
    ));

    let inline_pipe_config = database_config("builtin_model_inline_payload_extra_pipe");
    let model_path = AssetPath::parse("models/extra_pipe_inline.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body|v 0 0 0|v 1 0 0|v 0 1 0|i 0 1 2\n".to_vec(),
    );
    let mut database = AssetDatabase::new(inline_pipe_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/extra_pipe_inline.model")
                && message.contains("model mesh on line 2 inline payload must use exactly one `|` separator")
    ));

    let trailing_empty_config = database_config("builtin_model_dependency_trailing_empty_label");
    let model_path = AssetPath::parse("models/trailing_empty_dependency.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\ndepends=mesh:Body,\nname=hero\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(trailing_empty_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/trailing_empty_dependency.model")
                && message.contains("model dependency list")
                && message.contains("contains an empty generated label")
    ));

    let middle_empty_config = database_config("builtin_model_materials_middle_empty_label");
    let model_path = AssetPath::parse("models/middle_empty_material_binding.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nmaterials=HeroMaterial,,AltMaterial\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(middle_empty_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/middle_empty_material_binding.model")
                && message.contains("model dependency list")
                && message.contains("contains an empty generated label")
    ));
}

#[test]
fn database_model_importer_validates_standalone_generated_payloads() {
    let skeleton_config = database_config("builtin_model_standalone_invalid_skeleton_payload");
    let model_path = AssetPath::parse("models/invalid_standalone_skeleton.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nbone=Root\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(skeleton_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/invalid_standalone_skeleton.model")
                && message.contains("model skeleton `Rig`")
                && message.contains("payload is invalid")
                && message.contains("duplicates an earlier bone name")
    ));

    let animation_config = database_config("builtin_model_standalone_invalid_animation_payload");
    let model_path = AssetPath::parse("models/invalid_standalone_animation.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nanimation=Idle\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(animation_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/invalid_standalone_animation.model")
                && message.contains("model animation `Idle`")
                && message.contains("payload is invalid")
                && message.contains("animation source must contain at least one track")
    ));

    let material_config = database_config("builtin_model_standalone_invalid_material_payload");
    let model_path = AssetPath::parse("models/invalid_standalone_material.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmaterial=HeroMaterial\nname=hero\nalpha_mode=screen\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(material_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/invalid_standalone_material.model")
                && message.contains("model material `HeroMaterial`")
                && message.contains("payload is invalid")
                && message.contains("invalid material alpha mode `screen`")
    ));
}

#[test]
fn database_model_importer_rejects_duplicate_generated_dependency_metadata() {
    let duplicate_depends_config = database_config("builtin_model_duplicate_depends_metadata");
    let model_path = AssetPath::parse("models/duplicate_depends.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\ndepends=mesh:Body,mesh:Body\nname=hero\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(duplicate_depends_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/duplicate_depends.model")
                && message.contains("model material `HeroMaterial`")
                && message.contains("repeats generated dependency `Body`")
    ));

    let duplicate_material_config =
        database_config("builtin_model_duplicate_mesh_material_binding");
    let model_path = AssetPath::parse("models/duplicate_mesh_material.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nmaterials=HeroMaterial,HeroMaterial\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\nname=hero\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(duplicate_material_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/duplicate_mesh_material.model")
                && message.contains("model mesh `Body`")
                && message.contains("repeats generated dependency `HeroMaterial`")
    ));

    let duplicate_physics_config = database_config("builtin_model_duplicate_mesh_physics_binding");
    let model_path = AssetPath::parse("models/duplicate_mesh_physics.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nphysics_meshes=Collision,Collision\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nphysics_mesh=Collision\nNGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(duplicate_physics_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/duplicate_mesh_physics.model")
                && message.contains("model mesh `Body`")
                && message.contains("repeats generated dependency `Collision`")
    ));
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_generates_physics_mesh_subresources() {
    let config = database_config("builtin_model_physics_mesh_subresource");
    let model_path = AssetPath::parse("models/collision_model.model");
    let mesh_path = AssetPath::parse("models/collision_model.Body.mesh");
    let physics_path = AssetPath::parse("models/collision_model.Collision.physics");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nphysics_mesh=Collision\ndepends=mesh:Body\nNGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let physics_metadata = database.registry().metadata_by_path(&physics_path).unwrap();
    let physics_id = physics_metadata.id;

    assert_eq!(physics_metadata.asset_type, PhysicsMesh::TYPE_ID);
    assert_eq!(physics_metadata.dependencies, vec![mesh_id]);
    assert_eq!(physics_metadata.labels, vec!["Collision"]);
    assert_eq!(physics_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, physics_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(physics_path.path())).unwrap(),
        b"NGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\n"
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&physics_path)
            .unwrap()
            .dependencies,
        vec![mesh_id]
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let physics_output = database
        .cook_asset(physics_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "physics_model",
            vec![mesh_id, physics_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(physics_id),
        Some([mesh_id].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);
    assert_eq!(
        reader.read_path(&physics_path).unwrap(),
        physics_output.bytes
    );

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(physics_id), AssetLoadState::Ready);
    assert_eq!(
        server.get_by_id::<PhysicsMesh>(physics_id).unwrap().indices,
        vec![[0, 1, 2]]
    );

    let filtered_config = database_config("builtin_model_filter_physics_mesh_subresource");
    let model_path = AssetPath::parse("models/filtered_collision.model");
    let mesh_path = AssetPath::parse("models/filtered_collision.Body.mesh");
    let physics_path = AssetPath::parse("models/filtered_collision.Collision.physics");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\ndepends=physics_mesh:Collision\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nphysics_mesh=Collision\nNGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(filtered_config);
    database.set_io(io);
    database.register_builtin_importers();

    let mut settings = ImporterSettings::default();
    settings.set("import_physics_meshes", "false");
    let model_id = database
        .import_asset_path_with_settings(&model_path, &settings)
        .unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();

    assert!(database
        .registry()
        .metadata_by_path(&physics_path)
        .is_none());
    assert!(mesh_metadata.dependencies.is_empty());
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_metadata.id]
    );

    let invalid_config = database_config("builtin_model_invalid_physics_mesh_subresource");
    let model_path = AssetPath::parse("models/invalid_collision.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nphysics_mesh=Collision\nNGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(invalid_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/invalid_collision.model")
                && message.contains("model physics_mesh `Collision`")
                && message.contains("payload is invalid")
                && message.contains("physics mesh index 1 references missing vertex")
    ));
}

#[test]
fn database_model_importer_records_mesh_physics_mesh_binding_metadata() {
    let config = database_config("builtin_model_mesh_physics_mesh_binding");
    let model_path = AssetPath::parse("models/mesh_collision_binding.model");
    let mesh_path = AssetPath::parse("models/mesh_collision_binding.Body.mesh");
    let collision_path = AssetPath::parse("models/mesh_collision_binding.Collision.physics");
    let proxy_path = AssetPath::parse("models/mesh_collision_binding.Proxy.physics");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nphysics_meshes=Collision,Proxy\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nphysics_mesh=Collision\nNGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nphysics_mesh=Proxy\nNGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 2 0 0\nv 0 2 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;
    let collision_id = database
        .registry()
        .metadata_by_path(&collision_path)
        .unwrap()
        .id;
    let proxy_id = database
        .registry()
        .metadata_by_path(&proxy_path)
        .unwrap()
        .id;

    assert_eq!(mesh_metadata.dependencies, vec![collision_id, proxy_id]);
    assert_eq!(mesh_metadata.labels, vec!["Body"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, collision_id, proxy_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&mesh_path)
            .unwrap()
            .dependencies,
        vec![collision_id, proxy_id]
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let collision_output = database
        .cook_asset(collision_id, TargetPlatform::Windows)
        .unwrap();
    let proxy_output = database
        .cook_asset(proxy_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "mesh_collision_binding",
            vec![mesh_id, collision_id, proxy_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([collision_id, proxy_id].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);
    assert_eq!(
        reader.read_path(&collision_path).unwrap(),
        collision_output.bytes
    );
    assert_eq!(reader.read_path(&proxy_path).unwrap(), proxy_output.bytes);

    let unknown_config = database_config("builtin_model_unknown_mesh_physics_mesh_binding");
    let model_path = AssetPath::parse("models/unknown_mesh_collision.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nphysics_mesh=Missing\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(unknown_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/unknown_mesh_collision.model")
                && message.contains("model mesh `Body`")
                && message.contains("references unknown physics mesh `Missing`")
    ));

    let wrong_kind_config = database_config("builtin_model_mesh_physics_mesh_binding_wrong_kind");
    let model_path = AssetPath::parse("models/wrong_kind_mesh_collision.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nphysics_mesh=HeroMaterial\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\nname=hero\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(wrong_kind_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/wrong_kind_mesh_collision.model")
                && message.contains("physics mesh binding `HeroMaterial`")
                && message.contains("references generated material `HeroMaterial` instead of a physics_mesh")
    ));
}

#[test]
fn database_model_importer_records_physics_mesh_target_mesh_metadata() {
    let config = database_config("builtin_model_physics_mesh_target_mesh");
    let model_path = AssetPath::parse("models/physics_target_mesh.model");
    let mesh_path = AssetPath::parse("models/physics_target_mesh.Body.mesh");
    let physics_path = AssetPath::parse("models/physics_target_mesh.Collision.physics");
    let model_source = b"NGA_MODEL_V1
mesh=Body
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
physics_mesh=Collision
target_mesh=Body
NGA_PHYSICS_MESH_V1
kind=trimesh
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let physics_metadata = database.registry().metadata_by_path(&physics_path).unwrap();
    let physics_id = physics_metadata.id;

    assert_eq!(physics_metadata.dependencies, vec![mesh_id]);
    assert_eq!(physics_metadata.labels, vec!["Collision"]);
    assert_eq!(physics_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, physics_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&physics_path)
            .unwrap()
            .dependencies,
        vec![mesh_id]
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let physics_output = database
        .cook_asset(physics_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "physics_target_mesh",
            vec![mesh_id, physics_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(physics_id),
        Some([mesh_id].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);
    assert_eq!(
        reader.read_path(&physics_path).unwrap(),
        physics_output.bytes
    );

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(physics_id),
        vec![mesh_id]
    );
    assert_eq!(server.state_by_id(mesh_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(physics_id), AssetLoadState::Ready);

    let unknown_config = database_config("builtin_model_physics_target_unknown_mesh");
    let model_path = AssetPath::parse("models/unknown_physics_target.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nphysics_mesh=Collision\ntarget_mesh=Missing\nNGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(unknown_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/unknown_physics_target.model")
                && message.contains("model physics_mesh `Collision`")
                && message.contains("references unknown target mesh `Missing`")
    ));

    let wrong_kind_config = database_config("builtin_model_physics_target_wrong_kind");
    let model_path = AssetPath::parse("models/wrong_kind_physics_target.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmaterial=HeroMaterial\nname=hero\nend\nphysics_mesh=Collision\ntarget_mesh=HeroMaterial\nNGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(wrong_kind_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/wrong_kind_physics_target.model")
                && message.contains("target mesh `HeroMaterial`")
                && message.contains("references generated material `HeroMaterial` instead of a mesh")
    ));

    let repeated_config = database_config("builtin_model_physics_target_repeated");
    let model_path = AssetPath::parse("models/repeated_physics_target.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nphysics_mesh=Collision\nmesh=Body\ntarget_mesh=Body\nNGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(repeated_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/repeated_physics_target.model")
                && message.contains("model physics_mesh `Collision`")
                && message.contains("repeats target mesh metadata")
    ));
}

#[test]
fn database_model_importer_records_material_mesh_target_metadata() {
    let config = database_config("builtin_model_material_mesh_target");
    let model_path = AssetPath::parse("models/material_mesh.model");
    let mesh_path = AssetPath::parse("models/material_mesh.Body.mesh");
    let material_path = AssetPath::parse("models/material_mesh.HeroMaterial.material");
    let model_source = b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\ntarget_mesh=Body\nname=hero\nbase_color=0.4,0.5,0.6,1\nend\n".to_vec();
    let expected_material = b"name=hero\nbase_color=0.4,0.5,0.6,1\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.dependencies, vec![mesh_id]);
    assert_eq!(material_metadata.labels, vec!["HeroMaterial"]);
    assert_eq!(material_metadata.importer_version, 111);
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![mesh_id]
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let material_output = database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_mesh_model",
            vec![mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([mesh_id].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);
    assert_eq!(
        reader.read_path(&material_path).unwrap(),
        material_output.bytes
    );

    let unknown_config = database_config("builtin_model_unknown_material_mesh_target");
    let model_path = AssetPath::parse("models/unknown_material_mesh.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmaterial=HeroMaterial\nmesh=Missing\nname=hero\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(unknown_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/unknown_material_mesh.model")
                && message.contains("model material `HeroMaterial`")
                && message.contains("references unknown target mesh `Missing`")
    ));

    let wrong_kind_config = database_config("builtin_model_material_mesh_target_wrong_kind");
    let model_path = AssetPath::parse("models/wrong_kind_material_mesh.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\nmaterial=HeroMaterial\ntarget_mesh=Rig\nname=hero\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(wrong_kind_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/wrong_kind_material_mesh.model")
                && message.contains("target mesh `Rig`")
                && message.contains("references generated skeleton `Rig` instead of a mesh")
    ));

    let repeated_config = database_config("builtin_model_repeated_material_mesh_target");
    let model_path = AssetPath::parse("models/repeated_material_mesh.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\nmesh=Body\ntarget_mesh=Body\nname=hero\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(repeated_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/repeated_material_mesh.model")
                && message.contains("repeats target mesh metadata")
    ));
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_records_manifest_mesh_material_dependency() {
    let config = database_config("builtin_model_manifest_mesh_material_dependency");
    let shader_path = AssetPath::parse("shaders/pbr.wgsl");
    let texture_path = AssetPath::parse("textures/albedo.texture");
    let model_path = AssetPath::parse("models/mesh_material.model");
    let mesh_path = AssetPath::parse("models/mesh_material.Body.mesh");
    let material_path = AssetPath::parse("models/mesh_material.HeroMaterial.material");
    let model_source = b"NGA_MODEL_V1\nmesh=Body\nmaterial=HeroMaterial\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmaterial=HeroMaterial\nname=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=0.9,0.8,0.7,1\nend\n".to_vec();
    let expected_material = b"name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=0.9,0.8,0.7,1\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(shader_path.path(), shader_bytes());
    io.insert(texture_path.path(), texture_bytes(1, 1, 88));
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let shader_id = database.import_asset_path(&shader_path).unwrap();
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_id = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .id;

    assert_eq!(
        database
            .registry()
            .metadata_by_path(&mesh_path)
            .unwrap()
            .dependencies,
        vec![material_id]
    );
    assert_eq!(
        database
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![shader_id, texture_id]
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![shader_id, texture_id, mesh_id, material_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&mesh_path)
            .unwrap()
            .dependencies,
        vec![material_id]
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let expected_cooked_mesh = simple_binary_mesh_bytes();
    assert_eq!(mesh_output.bytes, expected_cooked_mesh);
    let material_output = database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(shader_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(texture_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "mesh_material_model",
            vec![mesh_id, material_id, shader_id, texture_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([shader_id, texture_id].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), expected_cooked_mesh);
    assert_eq!(
        reader.read_path(&material_path).unwrap(),
        material_output.bytes
    );

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(mesh_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(material_id), AssetLoadState::Ready);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_records_skinned_mesh_skeleton_dependency() {
    let config = database_config("builtin_model_skinned_mesh_dependency");
    let model_path = AssetPath::parse("models/skinned.model");
    let mesh_path = AssetPath::parse("models/skinned.Body.mesh");
    let skeleton_path = AssetPath::parse("models/skinned.Rig.skeleton");
    let model_source = b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 1 0 0\nj 1 0 0 0\nj 0 0 0 0\nw 0.75 0.25 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root;bind=1,0,0,2,0,1,0,0,0,0,1,0,0,0,0,1;inverse_bind=1,0,0,-2,0,1,0,0,0,0,1,0,0,0,0,1\nbone=Spine;parent=0\nend\n".to_vec();
    let expected_mesh = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 1 0 0\nj 1 0 0 0\nj 0 0 0 0\nw 0.75 0.25 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\n".to_vec();
    let expected_skeleton = b"NGA_SKELETON_V1\nbone=Root;bind=1,0,0,2,0,1,0,0,0,0,1,0,0,0,0,1;inverse_bind=1,0,0,-2,0,1,0,0,0,0,1,0,0,0,0,1\nbone=Spine;parent=0\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;
    let skeleton_metadata = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap();
    let skeleton_id = skeleton_metadata.id;

    assert_eq!(mesh_metadata.dependencies, vec![skeleton_id]);
    assert_eq!(mesh_metadata.labels, vec!["Body"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(skeleton_metadata.labels, vec!["Rig"]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, skeleton_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    assert_eq!(
        fs::read(config.imported_root.join(skeleton_path.path())).unwrap(),
        expected_skeleton
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&mesh_path)
            .unwrap()
            .dependencies,
        vec![skeleton_id]
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let expected_cooked_mesh = skinned_binary_mesh_bytes();
    assert_eq!(mesh_output.bytes, expected_cooked_mesh);
    database
        .cook_asset(skeleton_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "skinned_model",
            vec![mesh_id, skeleton_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([skeleton_id].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), expected_cooked_mesh);
    assert_eq!(reader.read_path(&skeleton_path).unwrap(), expected_skeleton);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(mesh.joints[0], [0, 1, 0, 0]);
    let skeleton = server.get_by_id::<Skeleton>(skeleton_id).unwrap();
    assert_eq!(skeleton.bones.len(), 2);
    assert_eq!(skeleton.bones[0].local_bind_transform[0][3], 2.0);
    assert_eq!(skeleton.inverse_bind_poses[0][0][3], -2.0);
}

#[test]
fn database_model_importer_validates_skin_root_bone_metadata() {
    let config = database_config("builtin_model_skin_root_bone");
    let model_path = AssetPath::parse("models/skin_root.model");
    let mesh_path = AssetPath::parse("models/skin_root.Body.mesh");
    let skeleton_path = AssetPath::parse("models/skin_root.Rig.skeleton");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nskin_root=Spine\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 1 2 0 0\nj 2 1 0 0\nj 2 0 0 0\nw 0.5 0.5 0 0\nw 0.75 0.25 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nbone=Spine;parent=0\nbone=Arm;parent=1\nbone=Prop;parent=0\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let skeleton_id = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap()
        .id;

    assert_eq!(mesh_metadata.dependencies, vec![skeleton_id]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_metadata.id, skeleton_id]
    );

    let missing_root_config = database_config("builtin_model_missing_skin_root_bone");
    let model_path = AssetPath::parse("models/missing_skin_root.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nskin_root=Missing\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 1 0 0 0\nj 1 0 0 0\nj 1 0 0 0\nw 1 0 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nbone=Spine;parent=0\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(missing_root_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/missing_skin_root.model")
                && message.contains("skin root bone `Missing` is missing from skeleton `Rig`")
    ));

    let outside_root_config = database_config("builtin_model_skin_joint_outside_root_bone");
    let model_path = AssetPath::parse("models/outside_skin_root.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nskin_root=Spine\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 3 0 0 0\nj 2 0 0 0\nj 1 0 0 0\nw 1 0 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nbone=Spine;parent=0\nbone=Arm;parent=1\nbone=Prop;parent=0\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(outside_root_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/outside_skin_root.model")
                && message.contains(
                    "skin joint 3 at vertex 0 targets bone `Prop` outside skin root `Spine`"
                )
    ));

    let no_skin_config = database_config("builtin_model_skin_root_without_skin");
    let model_path = AssetPath::parse("models/skin_root_without_skin.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nroot_bone=Root\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(no_skin_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/skin_root_without_skin.model")
                && message.contains("declares skin root bone metadata without skin skeleton metadata")
    ));

    let repeated_config = database_config("builtin_model_repeated_skin_root_bone");
    let model_path = AssetPath::parse("models/repeated_skin_root.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nskin_root=Root\nroot_bone=Root\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(repeated_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/repeated_skin_root.model")
                && message.contains("repeats skin root bone metadata")
    ));
}

#[test]
fn database_model_importer_requires_skin_root_for_multi_root_skeletons() {
    let config = database_config("builtin_model_multi_root_skin_root_scope");
    let model_path = AssetPath::parse("models/multi_root_skin_root.model");
    let mesh_path = AssetPath::parse("models/multi_root_skin_root.Body.mesh");
    let skeleton_path = AssetPath::parse("models/multi_root_skin_root.Rig.skeleton");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nskin_root=RootA\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 0 0 0\nj 0 0 0 0\nj 0 0 0 0\nw 1 0 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=RootA\nbone=RootB\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let skeleton_id = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap()
        .id;

    assert_eq!(mesh_metadata.dependencies, vec![skeleton_id]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_metadata.id, skeleton_id]
    );

    let missing_root_scope_config = database_config("builtin_model_multi_root_skin_missing_scope");
    let model_path = AssetPath::parse("models/multi_root_skin_missing_scope.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 0 0 0\nj 0 0 0 0\nj 0 0 0 0\nw 1 0 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=RootA\nbone=RootB\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(missing_root_scope_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/multi_root_skin_missing_scope.model")
                && message.contains("skin skeleton `Rig` has multiple root bones (RootA, RootB)")
                && message.contains("declare skin_root, root_bone, or skin_root_bone metadata")
    ));
}

#[test]
fn database_model_importer_validates_skin_joint_limit_metadata() {
    let config = database_config("builtin_model_skin_joint_limit");
    let model_path = AssetPath::parse("models/skin_joint_limit.model");
    let mesh_path = AssetPath::parse("models/skin_joint_limit.Body.mesh");
    let skeleton_path = AssetPath::parse("models/skin_joint_limit.Rig.skeleton");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nmax_skin_joints=2\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 1 0 0\nj 1 0 0 0\nj 0 0 0 0\nw 0.5 0.5 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nbone=Spine;parent=0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let skeleton_id = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap()
        .id;

    assert_eq!(mesh_metadata.dependencies, vec![skeleton_id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_metadata.id, skeleton_id]
    );

    let low_limit_config = database_config("builtin_model_skin_joint_limit_too_low");
    let model_path = AssetPath::parse("models/low_skin_joint_limit.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nskin_joint_limit=1\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 1 0 0\nj 1 0 0 0\nj 0 0 0 0\nw 0.5 0.5 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nbone=Spine;parent=0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(low_limit_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/low_skin_joint_limit.model")
                && message.contains("skin skeleton `Rig` has 2 bones")
                && message.contains("exceeds declared skin joint limit 1")
    ));

    let no_skin_config = database_config("builtin_model_skin_joint_limit_without_skin");
    let model_path = AssetPath::parse("models/skin_joint_limit_without_skin.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nmax_skin_joints=4\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(no_skin_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/skin_joint_limit_without_skin.model")
                && message.contains("declares skin joint limit metadata without skin skeleton metadata")
    ));

    let zero_config = database_config("builtin_model_zero_skin_joint_limit");
    let model_path = AssetPath::parse("models/zero_skin_joint_limit.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nmax_skin_joints=0\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(zero_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/zero_skin_joint_limit.model")
                && message.contains("skin joint limit on line 4 must be greater than zero")
    ));

    let repeated_config = database_config("builtin_model_repeated_skin_joint_limit");
    let model_path = AssetPath::parse("models/repeated_skin_joint_limit.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nmax_skin_joints=4\nskin_joint_limit=4\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(repeated_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/repeated_skin_joint_limit.model")
                && message.contains("repeats skin joint limit metadata")
    ));
}

#[test]
fn database_model_importer_validates_skin_influence_limit_metadata() {
    let config = database_config("builtin_model_skin_influence_limit");
    let model_path = AssetPath::parse("models/skin_influence_limit.model");
    let mesh_path = AssetPath::parse("models/skin_influence_limit.Body.mesh");
    let skeleton_path = AssetPath::parse("models/skin_influence_limit.Rig.skeleton");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nskin_influence_limit=2\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 1 0 0\nj 1 0 0 0\nj 0 0 0 0\nw 0.75 0.25 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nbone=Spine;parent=0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let skeleton_id = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap()
        .id;

    assert_eq!(mesh_metadata.dependencies, vec![skeleton_id]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_metadata.id, skeleton_id]
    );

    let too_low_config = database_config("builtin_model_skin_influence_limit_too_low");
    let model_path = AssetPath::parse("models/low_skin_influence_limit.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nmax_skin_influences=1\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 1 0 0\nj 1 0 0 0\nj 0 0 0 0\nw 0.75 0.25 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nbone=Spine;parent=0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(too_low_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/low_skin_influence_limit.model")
                && message.contains("skin weights at vertex 0")
                && message.contains("use 2 influences")
                && message.contains("exceeds declared skin influence limit 1")
    ));

    let no_skin_config = database_config("builtin_model_skin_influence_limit_without_skin");
    let model_path = AssetPath::parse("models/skin_influence_limit_without_skin.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin_influence_limit=2\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(no_skin_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/skin_influence_limit_without_skin.model")
                && message.contains("declares skin influence limit metadata without skin skeleton metadata")
    ));

    let zero_config = database_config("builtin_model_zero_skin_influence_limit");
    let model_path = AssetPath::parse("models/zero_skin_influence_limit.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nskin_influence_limit=0\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(zero_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/zero_skin_influence_limit.model")
                && message.contains("skin influence limit on line 4 must be greater than zero")
    ));

    let high_config = database_config("builtin_model_high_skin_influence_limit");
    let model_path = AssetPath::parse("models/high_skin_influence_limit.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nmax_skin_influences=5\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(high_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/high_skin_influence_limit.model")
                && message.contains("skin influence limit on line 4 must not exceed 4")
    ));

    let repeated_config = database_config("builtin_model_repeated_skin_influence_limit");
    let model_path = AssetPath::parse("models/repeated_skin_influence_limit.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nmax_skin_influences=2\nskin_influence_limit=2\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(repeated_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/repeated_skin_influence_limit.model")
                && message.contains("repeats skin influence limit metadata")
    ));
}

#[test]
fn database_model_importer_reports_generated_path_collisions() {
    let config = database_config("builtin_model_generated_path_collision");
    let model_path = AssetPath::parse("models/collision.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body_LOD0\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nmesh=Body/LOD0\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/collision.model")
                && message.contains("model generated mesh `Body/LOD0`")
                && message.contains("duplicate generated path `models/collision.Body_LOD0.mesh`")
                && message.contains("first declared by mesh `Body_LOD0`")
    ));
}

#[test]
fn database_model_importer_reports_invalid_skin_binding_metadata() {
    let unknown_config = database_config("builtin_model_unknown_skin_skeleton");
    let model_path = AssetPath::parse("models/unknown_skin.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Missing\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(unknown_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/unknown_skin.model")
                && message.contains("references unknown skin skeleton `Missing`")
    ));

    let range_config = database_config("builtin_model_skin_joint_range");
    let model_path = AssetPath::parse("models/bad_skin.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 2 0 0\nj 1 0 0 0\nj 0 0 0 0\nw 1 0 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nbone=Spine;parent=0\nend\n".to_vec(),
    );
    let mut database = AssetDatabase::new(range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_skin.model")
                && message.contains("skin joint 2 at vertex 0 references missing skeleton bone")
                && message.contains("skeleton `Rig` has 2 bones")
    ));

    let invalid_inverse_bind_config = database_config("builtin_model_skin_bad_inverse_bind");
    let model_path = AssetPath::parse("models/bad_inverse_bind_skin.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 0 0 0\nj 0 0 0 0\nj 0 0 0 0\nw 1 0 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root;bind=1,0,0,2,0,1,0,0,0,0,1,0,0,0,0,1;inverse_bind=1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(invalid_inverse_bind_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_inverse_bind_skin.model")
                && message.contains("skin skeleton `Rig` is invalid")
                && message.contains("inverse_bind on line 2 does not invert bind pose for bone `Root`")
    ));

    let missing_skin_attributes_config = database_config("builtin_model_skin_without_attributes");
    let model_path = AssetPath::parse("models/missing_skin_attributes.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(missing_skin_attributes_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/missing_skin_attributes.model")
                && message.contains("declares skin skeleton `Rig`")
                && message.contains("has no skin joint/weight attributes")
    ));

    let zero_weight_config = database_config("builtin_model_skin_zero_weight_total");
    let model_path = AssetPath::parse("models/zero_skin_weights.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 0 0 0\nj 0 0 0 0\nj 0 0 0 0\nw 0 0 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(zero_weight_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/zero_skin_weights.model")
                && message.contains("skin weights at vertex 0")
                && message.contains("must have a positive total")
    ));

    let unnormalized_weight_config = database_config("builtin_model_skin_unnormalized_weights");
    let model_path = AssetPath::parse("models/unnormalized_skin_weights.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 0 0 0\nj 0 0 0 0\nj 0 0 0 0\nw 2 0 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(unnormalized_weight_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/unnormalized_skin_weights.model")
                && message.contains("skin weights at vertex 0")
                && message.contains("must sum to 1.0")
    ));
}

#[test]
fn database_model_importer_rejects_duplicate_active_skin_joints() {
    let config = database_config("builtin_model_duplicate_active_skin_joint");
    let model_path = AssetPath::parse("models/duplicate_active_skin_joint.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nskin=Rig\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 0 1 0\nj 1 0 0 0\nj 0 0 0 0\nw 0.5 0.5 0 0\nw 1 0 0 0\nw 1 0 0 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nbone=Spine;parent=0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/duplicate_active_skin_joint.model")
                && message.contains("skin joint 0 appears more than once with active weights at vertex 0")
                && message.contains("skeleton `Rig`")
    ));
}

#[test]
fn database_model_importer_reports_invalid_mesh_material_binding_metadata() {
    let unknown_config = database_config("builtin_model_unknown_mesh_material");
    let model_path = AssetPath::parse("models/unknown_mesh_material.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nmaterial=MissingMaterial\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(unknown_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/unknown_mesh_material.model")
                && message.contains("model mesh `Body`")
                && message.contains("references unknown material `MissingMaterial`")
    ));

    let wrong_kind_config = database_config("builtin_model_mesh_material_wrong_kind");
    let model_path = AssetPath::parse("models/wrong_kind_mesh_material.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nmaterial=Rig\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(wrong_kind_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/wrong_kind_mesh_material.model")
                && message.contains("material binding `Rig`")
                && message.contains("references generated skeleton `Rig` instead of a material")
    ));
}

#[test]
fn database_model_importer_validates_animation_skeleton_targets() {
    let missing_bone_config = database_config("builtin_model_animation_missing_bone_target");
    let model_path = AssetPath::parse("models/bad_animation_bone.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\nanimation=Walk\ndepends=Rig\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=bone:Missing\ntranslation=0:0,0,0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(missing_bone_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_animation_bone.model")
                && message.contains("model animation `Walk`")
                && message.contains("track 0 targets missing skeleton bone `Missing`")
                && message.contains("skeleton `Rig`")
    ));

    let missing_node_config = database_config("builtin_model_animation_missing_node_target");
    let model_path = AssetPath::parse("models/bad_animation_node.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\nanimation=Walk\ndepends=Rig\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=node:Missing\ntranslation=0:0,0,0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(missing_node_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_animation_node.model")
                && message.contains("model animation `Walk`")
                && message.contains("track 0 targets missing skeleton node `Missing`")
                && message.contains("skeleton `Rig`")
    ));

    let node_index_config = database_config("builtin_model_animation_bad_node_index");
    let model_path = AssetPath::parse("models/bad_animation_node_index.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\nanimation=Walk\ndepends=Rig\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=node_index:1\ntranslation=0:0,0,0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(node_index_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_animation_node_index.model")
                && message.contains("model animation `Walk`")
                && message.contains("track 0 node_index 1 references missing skeleton bone")
                && message.contains("skeleton `Rig` has 1 bones")
    ));

    let keyframe_time_config = database_config("builtin_model_animation_bad_keyframe_time");
    let model_path = AssetPath::parse("models/bad_animation_keyframe_time.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\nanimation=Walk\ndepends=Rig\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=bone:Root\ntranslation=2:0,0,0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(keyframe_time_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_animation_keyframe_time.model")
                && message.contains("model animation `Walk`")
                && message.contains("payload is invalid")
                && message.contains("translation keyframe 0 in track 0 has time 2 beyond duration 1")
    ));
}

#[test]
fn database_model_importer_validates_animation_track_shape() {
    let empty_track_config = database_config("builtin_model_animation_empty_track");
    let model_path = AssetPath::parse("models/bad_animation_empty_track.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\nanimation=Walk\nskeleton=Rig\n---\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=bone:Root\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(empty_track_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_animation_empty_track.model")
                && message.contains("model animation `Walk`")
                && message.contains("payload is invalid")
                && message.contains(
                    "animation track 0 must contain at least one translation, rotation, or scale keyframe"
                )
    ));

    let duplicate_target_config = database_config("builtin_model_animation_duplicate_target");
    let model_path = AssetPath::parse("models/bad_animation_duplicate_target.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\nanimation=Walk\nskeleton=Rig\n---\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=node:Root\ntranslation=0:0,0,0\ntrack=node:Root\nrotation=0:0,0,0,1\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(duplicate_target_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_animation_duplicate_target.model")
                && message.contains("model animation `Walk`")
                && message.contains("payload is invalid")
                && message.contains("animation track 1 duplicates target `node:Root` from track 0")
    ));
}

#[test]
fn database_model_importer_records_animation_skeleton_metadata() {
    let config = database_config("builtin_model_animation_skeleton_metadata");
    let model_path = AssetPath::parse("models/animated.model");
    let skeleton_path = AssetPath::parse("models/animated.Rig.skeleton");
    let animation_path = AssetPath::parse("models/animated.Walk.animation");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nskeleton=Rig\nNGA_SKELETON_V1\nbone=Root\nend\nanimation=Walk\nskeleton=Rig\n---\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=node_index:0\ntranslation=0:0,0,0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let skeleton_id = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap()
        .id;
    let animation_metadata = database
        .registry()
        .metadata_by_path(&animation_path)
        .unwrap();

    assert_eq!(animation_metadata.labels, vec!["Walk"]);
    assert_eq!(animation_metadata.dependencies, vec![skeleton_id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![skeleton_id, animation_metadata.id]
    );

    let unknown_config = database_config("builtin_model_animation_unknown_skeleton_metadata");
    let model_path = AssetPath::parse("models/bad_animation_unknown_skeleton.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nanimation=Walk\nskeleton=Missing\n---\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=node_index:0\ntranslation=0:0,0,0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(unknown_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_animation_unknown_skeleton.model")
                && message.contains("model animation `Walk`")
                && message.contains("references unknown target skeleton `Missing`")
    ));

    let wrong_kind_config = database_config("builtin_model_animation_skeleton_wrong_kind");
    let model_path = AssetPath::parse("models/bad_animation_skeleton_wrong_kind.model");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nanimation=Walk\nskeleton=Body\n---\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=node_index:0\ntranslation=0:0,0,0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(wrong_kind_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_animation_skeleton_wrong_kind.model")
                && message.contains("target skeleton `Body`")
                && message.contains("references generated mesh `Body` instead of a skeleton")
    ));
}

#[test]
fn database_model_importer_can_filter_generated_skeleton_and_animation_subresources() {
    let config = database_config("builtin_model_filter_skeleton_and_animation_subresources");
    let model_path = AssetPath::parse("models/filtered_animation.model");
    let mesh_path = AssetPath::parse("models/filtered_animation.Body.mesh");
    let skeleton_path = AssetPath::parse("models/filtered_animation.Rig.skeleton");
    let animation_path = AssetPath::parse("models/filtered_animation.Walk.animation");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"NGA_MODEL_V1\nmesh=Body\n---\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\nend\nskeleton=Rig\n---\nNGA_SKELETON_V1\nbone=Root\nend\nanimation=Walk\nskeleton=Rig\n---\nNGA_ANIMATION_V1\nduration=1\nticks_per_second=24\ntrack=node_index:0\ntranslation=0:0,0,0\nend\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let mut settings = ImporterSettings::default();
    settings.set("import_skeleton", "false");
    settings.set("import_animations", "false");
    let model_id = database
        .import_asset_path_with_settings(&model_path, &settings)
        .unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();

    assert!(database
        .registry()
        .metadata_by_path(&skeleton_path)
        .is_none());
    assert!(database
        .registry()
        .metadata_by_path(&animation_path)
        .is_none());
    assert!(mesh_metadata.dependencies.is_empty());
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_metadata.id]
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_parses_obj_source_into_mesh_and_material_subresources() {
    let config = database_config("builtin_model_obj_source");
    let model_path = AssetPath::parse("models/prop.obj");
    let mesh_path = AssetPath::parse("models/prop.Prop.mesh");
    let material_path = AssetPath::parse("models/prop.Material_Red.material");
    let albedo_path = AssetPath::parse("models/textures/prop_albedo.texture");
    let normal_path = AssetPath::parse("models/textures/prop_normal.texture");
    let roughness_path = AssetPath::parse("models/textures/prop_roughness.texture");
    let metallic_path = AssetPath::parse("models/textures/prop_metallic.texture");
    let emissive_path = AssetPath::parse("models/textures/prop_emissive.texture");
    let model_source = b"# simple obj
MTLLIB prop.mtl
O Prop
V 0 0 0
V 1 0 0
V 1 1 0
V 0 1 0
VT 0 0
VT 1 0
VT 1 1
VT 0 1
VN 0 0 1
USEMTL Red
F 1/1/1 2/2/1 3/3/1 4/4/1
"
    .to_vec();
    let material_library_source = b"NewMtl Red
MAP_KD -BOOST 1.5 -BLENDU OFF -BlendV ON -CC TRUE -TEXRES 1024 -S 2 3 -O 0.25 0.5 -T 0.01 0.02 0.03 textures/prop_albedo.texture
MAP_NORMAL -BM 0.3 -COLORSPACE Non-Color textures/prop_normal.texture
MAP_PR textures/prop_roughness.texture -Clamp True
map_Pm -MM 0 1 textures/prop_metallic.texture
map_Ke -TYPE Sphere -IMFCHAN R textures/prop_emissive.texture
kD 0.8 0.2 0.1
D 0.75
nS 250
pM 0.5
kE 0.1 0.2 0.3
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 33);
    let normal_source = texture_bytes(1, 1, 44);
    let roughness_source = texture_bytes(1, 1, 55);
    let metallic_source = texture_bytes(1, 1, 66);
    let emissive_source = texture_bytes(1, 1, 77);
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
n 0 0 1
n 0 0 1
n 0 0 1
n 0 0 1
uv 0 0
uv 1 0
uv 1 1
uv 0 1
t 1 0 0 1
t 1 0 0 1
t 1 0 0 1
t 1 0 0 1
i 0 1 2
i 0 2 3
"
    .to_vec();
    let expected_material = b"# mtllib prop.mtl
name=Red
texture.albedo=models/textures/prop_albedo.texture
texture.albedo.transform.offset=0.25,0.5,0
texture.albedo.transform.scale=2,3,1
texture.albedo.transform.turbulence=0.01,0.02,0.03
texture.albedo.boost=1.5
texture.albedo.blend_u=false
texture.albedo.blend_v=true
texture.albedo.color_correction=true
texture.albedo.texture_resolution=1024
texture.normal=models/textures/prop_normal.texture
texture.normal.bump_scale=0.3
texture.normal.color_space=non_color
texture.roughness=models/textures/prop_roughness.texture
texture.roughness.sampler.address=clamp_to_edge
texture.metallic=models/textures/prop_metallic.texture
texture.metallic.color_remap=0,1
texture.emissive=models/textures/prop_emissive.texture
texture.emissive.source_channel=red
texture.emissive.projection=sphere
base_color=0.8,0.2,0.1,0.75
alpha_mode=blend
metallic=0.5
roughness=0.5
emissive=0.1,0.2,0.3
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/prop.mtl", material_library_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    io.insert(normal_path.path(), normal_source.clone());
    io.insert(roughness_path.path(), roughness_source.clone());
    io.insert(metallic_path.path(), metallic_source.clone());
    io.insert(emissive_path.path(), emissive_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let normal_id = database.import_asset_path(&normal_path).unwrap();
    let roughness_id = database.import_asset_path(&roughness_path).unwrap();
    let metallic_id = database.import_asset_path(&metallic_path).unwrap();
    let emissive_id = database.import_asset_path(&emissive_path).unwrap();
    let texture_ids = vec![albedo_id, normal_id, roughness_id, metallic_id, emissive_id];
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_id = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .id;

    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Prop"]);
    assert_eq!(mesh_metadata.dependencies, vec![material_id]);
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    assert_eq!(material_metadata.asset_type, AssetTypeId::of::<Material>());
    assert_eq!(material_metadata.labels, vec!["Material/Red"]);
    assert_eq!(material_metadata.dependencies, texture_ids);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![
            albedo_id,
            normal_id,
            roughness_id,
            metallic_id,
            emissive_id,
            mesh_id,
            material_id
        ]
    );

    for texture_id in [albedo_id, normal_id, roughness_id, metallic_id, emissive_id] {
        database
            .cook_asset(texture_id, TargetPlatform::Windows)
            .unwrap();
    }
    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let material_output = database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let expected_cooked_mesh = obj_binary_mesh_bytes();
    assert_eq!(mesh_output.bytes, expected_cooked_mesh);
    assert_eq!(mesh_output.version_hash, VersionHash(4));
    assert_eq!(material_output.bytes, expected_material);

    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "obj_model",
            vec![
                mesh_id,
                material_id,
                albedo_id,
                normal_id,
                roughness_id,
                metallic_id,
                emissive_id,
            ],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id, normal_id, roughness_id, metallic_id, emissive_id].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), expected_cooked_mesh);
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);
    assert_eq!(reader.read_path(&normal_path).unwrap(), normal_source);
    assert_eq!(reader.read_path(&roughness_path).unwrap(), roughness_source);
    assert_eq!(reader.read_path(&metallic_path).unwrap(), metallic_source);
    assert_eq!(reader.read_path(&emissive_path).unwrap(), emissive_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(mesh_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(material_id), AssetLoadState::Ready);
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.normals, vec![[0.0, 0.0, 1.0]; 4]);
    assert_eq!(
        mesh.uvs,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
    );
    assert_eq!(mesh.tangents, vec![[1.0, 0.0, 0.0, 1.0]; 4]);
    assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.name.as_deref(), Some("Red"));
    assert_eq!(material.textures.len(), 5);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(
        material.textures[0].options.transform.offset,
        [0.25, 0.5, 0.0]
    );
    assert_eq!(
        material.textures[0].options.transform.scale,
        [2.0, 3.0, 1.0]
    );
    assert_eq!(
        material.textures[0].options.transform.turbulence,
        [0.01, 0.02, 0.03]
    );
    assert_eq!(material.textures[0].options.boost, Some(1.5));
    assert_eq!(material.textures[0].options.blend_u, Some(false));
    assert_eq!(material.textures[0].options.blend_v, Some(true));
    assert_eq!(material.textures[0].options.color_correction, Some(true));
    assert_eq!(material.textures[0].options.texture_resolution, Some(1024));
    assert_eq!(material.textures[1].name, "normal");
    assert_eq!(material.textures[1].texture.id(), normal_id);
    assert_eq!(material.textures[1].options.bump_scale, Some(0.3));
    assert_eq!(
        material.textures[1].options.color_space,
        Some(MaterialTextureColorSpace::NonColor)
    );
    assert_eq!(material.textures[2].name, "roughness");
    assert_eq!(material.textures[2].texture.id(), roughness_id);
    assert_eq!(
        material.textures[2].sampler.address,
        AddressMode::ClampToEdge
    );
    assert_eq!(material.textures[3].name, "metallic");
    assert_eq!(material.textures[3].texture.id(), metallic_id);
    assert_eq!(material.textures[3].options.color_remap, Some([0.0, 1.0]));
    assert_eq!(material.textures[4].name, "emissive");
    assert_eq!(material.textures[4].texture.id(), emissive_id);
    assert_eq!(
        material.textures[4].options.source_channel,
        Some(MaterialTextureChannel::Red)
    );
    assert_eq!(
        material.textures[4].options.projection,
        Some(MaterialTextureProjection::Sphere)
    );
    assert_eq!(material.properties.base_color, [0.8, 0.2, 0.1, 0.75]);
    assert_eq!(material.render_state.alpha_mode, AlphaMode::Blend);
    assert_eq!(material.properties.metallic, 0.5);
    assert_eq!(material.properties.roughness, 0.5);
    assert_eq!(material.properties.emissive, [0.1, 0.2, 0.3]);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_parses_obj_inline_comments() {
    let config = database_config("builtin_model_obj_inline_comments");
    let model_path = AssetPath::parse("models/commented.model");
    let mesh_path = AssetPath::parse("models/commented.Panel.mesh");
    let material_path = AssetPath::parse("models/commented.Material_Red.material");
    let albedo_path = AssetPath::parse("models/textures/red.texture");
    let model_source = b"NGA_MODEL_OBJ_V1 # OBJ header comment
# comment-only line
mtllib commented.mtl # material library with a trailing comment
o Panel # object label comment
v 0 0 0 # origin
v 1 0 0 # right
v 0 1 0 # top
vt 0 0 # uv0
vt 1 0 # uv1
vt 0 1 # uv2
vn 0 0 1 # normal
usemtl Red # material binding comment
f 1/1/1 2/2/1 3/3/1 # triangle comment
"
    .to_vec();
    let material_source = b"newmtl Red # material name comment
Kd 0.2 0.3 0.4 # base color comment
map_Kd textures/red.texture # albedo texture comment
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 88);
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
n 0 0 1
n 0 0 1
n 0 0 1
uv 0 0
uv 1 0
uv 0 1
t 1 0 0 1
t 1 0 0 1
t 1 0 0 1
i 0 1 2
"
    .to_vec();
    let expected_material = b"# mtllib commented.mtl
name=Red
texture.albedo=models/textures/red.texture
base_color=0.2,0.3,0.4,1
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/commented.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Panel"]);
    assert_eq!(mesh_metadata.dependencies, vec![material_id]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    assert_eq!(material_metadata.asset_type, AssetTypeId::of::<Material>());
    assert_eq!(material_metadata.labels, vec!["Material/Red"]);
    assert_eq!(material_metadata.dependencies, vec![albedo_id]);
    assert_eq!(material_metadata.importer_version, 111);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![albedo_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&mesh_path)
            .unwrap()
            .dependencies,
        vec![material_id]
    );
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![albedo_id]
    );

    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let material_output = database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "commented_model",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);
    assert_eq!(
        reader.read_path(&material_path).unwrap(),
        material_output.bytes
    );
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(mesh_id),
        vec![material_id]
    );
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        vec![albedo_id]
    );
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(
        mesh.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(mesh.normals, vec![[0.0, 0.0, 1.0]; 3]);
    assert_eq!(mesh.uvs, vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    assert_eq!(mesh.tangents, vec![[1.0, 0.0, 0.0, 1.0]; 3]);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.name.as_deref(), Some("Red"));
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(material.properties.base_color, [0.2, 0.3, 0.4, 1.0]);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_generates_obj_smoothing_group_normals() {
    let config = database_config("builtin_model_obj_smoothing_group_normals");
    let model_path = AssetPath::parse("models/smooth.obj");
    let mesh_path = AssetPath::parse("models/smooth.Fold.mesh");
    let model_source = b"o Fold
v 0 0 0
v 1 0 0
v 0 1 0
v 0 0 1
S 1
F 1 2 3
F 1 3 4
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
v 0 0 1
n 0.70710677 0 0.70710677
n 0 0 1
n 0.70710677 0 0.70710677
n 1 0 0
i 0 1 2
i 0 2 3
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Fold"]);
    assert!(mesh_metadata.dependencies.is_empty());
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&mesh_path)
            .unwrap()
            .dependencies,
        Vec::<AssetId>::new()
    );

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "smooth_model",
            vec![mesh_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(reader.manifest().dependencies(mesh_id), Some([].as_slice()));

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(
        mesh.normals,
        vec![
            [0.70710677, 0.0, 0.70710677],
            [0.0, 0.0, 1.0],
            [0.70710677, 0.0, 0.70710677],
            [1.0, 0.0, 0.0],
        ]
    );
    assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_treats_obj_smoothing_off_case_insensitively() {
    let config = database_config("builtin_model_obj_smoothing_off_case");
    let model_path = AssetPath::parse("models/flat_case.obj");
    let mesh_path = AssetPath::parse("models/flat_case.Fold.mesh");
    let model_source = b"o Fold
v 0 0 0
v 1 0 0
v 0 1 0
v 0 0 1
S OFF
F 1 2 3
F 1 3 4
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
v 0 0 0
v 0 1 0
v 0 0 1
n 0 0 1
n 0 0 1
n 0 0 1
n 1 0 0
n 1 0 0
n 1 0 0
i 0 1 2
i 3 4 5
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Fold"]);
    assert!(mesh_metadata.dependencies.is_empty());
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "flat_case_model",
            vec![mesh_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(reader.manifest().dependencies(mesh_id), Some([].as_slice()));

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(mesh.vertices.len(), 6);
    assert_eq!(
        mesh.normals,
        vec![
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ]
    );
    assert_eq!(mesh.indices, vec![0, 1, 2, 3, 4, 5]);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_parses_obj_roughness_extensions() {
    let config = database_config("builtin_model_obj_roughness_extensions");
    let model_path = AssetPath::parse("models/rough.obj");
    let mesh_path = AssetPath::parse("models/rough.Panel.mesh");
    let material_path = AssetPath::parse("models/rough.Material_Rough.material");
    let roughness_path = AssetPath::parse("models/textures/spec_exponent.texture");
    let model_source = b"mtllib rough.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Rough
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Rough
pR 0.35
map_Ns -imfchan green -colorspace Non-Color textures/spec_exponent.texture
"
    .to_vec();
    let expected_material = b"# mtllib rough.mtl
name=Rough
texture.roughness=models/textures/spec_exponent.texture
texture.roughness.source_channel=green
texture.roughness.color_space=non_color
roughness=0.35
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/rough.mtl", material_source);
    io.insert(roughness_path.path(), texture_bytes(1, 1, 88));
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let roughness_id = database.import_asset_path(&roughness_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Rough"]);
    assert_eq!(material_metadata.dependencies, vec![roughness_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![roughness_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![roughness_id]
    );

    database
        .cook_asset(roughness_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "rough_model",
            vec![mesh_id, material_id, roughness_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([roughness_id].as_slice())
    );

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        vec![roughness_id]
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.properties.roughness, 0.35);
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "roughness");
    assert_eq!(material.textures[0].texture.id(), roughness_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Green)
    );
    assert_eq!(
        material.textures[0].options.color_space,
        Some(MaterialTextureColorSpace::NonColor)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_maps_obj_transparency_to_alpha_mode() {
    let config = database_config("builtin_model_obj_transparency_alpha_mode");
    let model_path = AssetPath::parse("models/translucent.obj");
    let mesh_path = AssetPath::parse("models/translucent.Panel.mesh");
    let material_path = AssetPath::parse("models/translucent.Material_Glass.material");
    let model_source = b"mtllib translucent.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Glass
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Glass
tR 0.4
"
    .to_vec();
    let expected_material = b"# mtllib translucent.mtl
name=Glass
base_color=1,1,1,0.6
alpha_mode=blend
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/translucent.mtl", material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Glass"]);
    assert!(material_metadata.dependencies.is_empty());
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        Vec::<AssetId>::new()
    );

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "translucent_model",
            vec![mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(&material.properties.base_color[..3], &[1.0, 1.0, 1.0]);
    assert!((material.properties.base_color[3] - 0.6).abs() < 0.0001);
    assert_eq!(material.render_state.alpha_mode, AlphaMode::Blend);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_maps_obj_dissolve_halo_to_alpha_mode() {
    let config = database_config("builtin_model_obj_dissolve_halo_alpha_mode");
    let model_path = AssetPath::parse("models/halo.obj");
    let mesh_path = AssetPath::parse("models/halo.Panel.mesh");
    let material_path = AssetPath::parse("models/halo.Material_Glass.material");
    let model_source = b"mtllib halo.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Glass
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Glass
d -HALO 0.4
"
    .to_vec();
    let expected_material = b"# mtllib halo.mtl
name=Glass
custom.dissolve_halo.bool=true
base_color=1,1,1,0.4
alpha_mode=blend
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/halo.mtl", material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Glass"]);
    assert!(material_metadata.dependencies.is_empty());
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        Vec::<AssetId>::new()
    );

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "halo_model",
            vec![mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(
        material.properties.custom.get("dissolve_halo"),
        Some(&MaterialPropertyValue::Bool(true))
    );
    assert!((material.properties.base_color[3] - 0.4).abs() < 0.0001);
    assert_eq!(material.render_state.alpha_mode, AlphaMode::Blend);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_sharpness_and_bump_alias() {
    let config = database_config("builtin_model_obj_sharpness_bump_alias");
    let model_path = AssetPath::parse("models/compat.obj");
    let mesh_path = AssetPath::parse("models/compat.Panel.mesh");
    let material_path = AssetPath::parse("models/compat.Material_Detail.material");
    let normal_path = AssetPath::parse("models/textures/detail_normal.texture");
    let model_source = b"mtllib compat.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Detail
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Detail
SHARPNESS 42
map_bump -bm 0.2 textures/detail_normal.texture
"
    .to_vec();
    let expected_material = b"# mtllib compat.mtl
name=Detail
texture.normal=models/textures/detail_normal.texture
texture.normal.bump_scale=0.2
custom.sharpness.float=42
"
    .to_vec();
    let normal_source = texture_bytes(1, 1, 36);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/compat.mtl", material_source);
    io.insert(normal_path.path(), normal_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let normal_id = database.import_asset_path(&normal_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Detail"]);
    assert_eq!(material_metadata.dependencies, vec![normal_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![normal_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![normal_id]
    );

    database
        .cook_asset(normal_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "compat_model",
            vec![mesh_id, material_id, normal_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([normal_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&normal_path).unwrap(), normal_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        vec![normal_id]
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(
        material.properties.custom.get("sharpness"),
        Some(&MaterialPropertyValue::Float(42.0))
    );
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "normal");
    assert_eq!(material.textures[0].texture.id(), normal_id);
    assert_eq!(material.textures[0].options.bump_scale, Some(0.2));
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_texture_antialiasing() {
    let config = database_config("builtin_model_obj_texture_antialiasing");
    let model_path = AssetPath::parse("models/antialias.obj");
    let mesh_path = AssetPath::parse("models/antialias.Panel.mesh");
    let material_path = AssetPath::parse("models/antialias.Material_Detail.material");
    let model_source = b"mtllib antialias.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Detail
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Detail
MAP_AAT OFF
"
    .to_vec();
    let expected_material = b"# mtllib antialias.mtl
name=Detail
custom.texture_antialias.bool=false
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/antialias.mtl", material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Detail"]);
    assert!(material_metadata.dependencies.is_empty());
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        Vec::<AssetId>::new()
    );

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "antialias_model",
            vec![mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(
        material.properties.custom.get("texture_antialias"),
        Some(&MaterialPropertyValue::Bool(false))
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_transmission_and_ior_texture_maps() {
    let config = database_config("builtin_model_obj_transmission_ior_texture_maps");
    let model_path = AssetPath::parse("models/optics.obj");
    let mesh_path = AssetPath::parse("models/optics.Panel.mesh");
    let material_path = AssetPath::parse("models/optics.Material_Glass.material");
    let transmission_path = AssetPath::parse("models/textures/glass_filter.texture");
    let ior_path = AssetPath::parse("models/textures/glass_ior.texture");
    let model_source = b"mtllib optics.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Glass
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Glass
map_Tf -imfchan g -colorspace Non-Color textures/glass_filter.texture
map_Ni -imfchan blue -colorspace Non-Color textures/glass_ior.texture
"
    .to_vec();
    let expected_material = b"# mtllib optics.mtl
name=Glass
texture.transmission_filter=models/textures/glass_filter.texture
texture.transmission_filter.source_channel=green
texture.transmission_filter.color_space=non_color
texture.index_of_refraction=models/textures/glass_ior.texture
texture.index_of_refraction.source_channel=blue
texture.index_of_refraction.color_space=non_color
"
    .to_vec();
    let transmission_source = texture_bytes(1, 1, 91);
    let ior_source = texture_bytes(1, 1, 92);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/optics.mtl", material_source);
    io.insert(transmission_path.path(), transmission_source.clone());
    io.insert(ior_path.path(), ior_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let transmission_id = database.import_asset_path(&transmission_path).unwrap();
    let ior_id = database.import_asset_path(&ior_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Glass"]);
    assert_eq!(
        material_metadata.dependencies,
        vec![transmission_id, ior_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![transmission_id, ior_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![transmission_id, ior_id]
    );

    database
        .cook_asset(transmission_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(ior_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "optics_model",
            vec![mesh_id, material_id, transmission_id, ior_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([transmission_id, ior_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(
        reader.read_path(&transmission_path).unwrap(),
        transmission_source
    );
    assert_eq!(reader.read_path(&ior_path).unwrap(), ior_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        vec![transmission_id, ior_id]
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 2);
    assert_eq!(material.textures[0].name, "transmission_filter");
    assert_eq!(material.textures[0].texture.id(), transmission_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Green)
    );
    assert_eq!(material.textures[1].name, "index_of_refraction");
    assert_eq!(material.textures[1].texture.id(), ior_id);
    assert_eq!(
        material.textures[1].options.source_channel,
        Some(MaterialTextureChannel::Blue)
    );
    assert_eq!(
        material.textures[1].options.color_space,
        Some(MaterialTextureColorSpace::NonColor)
    );
    assert_eq!(
        material.textures[0].options.color_space,
        Some(MaterialTextureColorSpace::NonColor)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_transmittance_aliases() {
    let config = database_config("builtin_model_obj_transmittance_aliases");
    let model_path = AssetPath::parse("models/transmittance_alias.obj");
    let mesh_path = AssetPath::parse("models/transmittance_alias.Pane.mesh");
    let material_path = AssetPath::parse("models/transmittance_alias.Material_Clear.material");
    let transmission_path = AssetPath::parse("models/textures/transmittance.texture");
    let model_source = b"mtllib transmittance_alias.mtl
o Pane
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Clear
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Clear
Kt 0.1 0.2 0.3
map_Kt -imfchan alpha textures/transmittance.texture
"
    .to_vec();
    let expected_material = b"# mtllib transmittance_alias.mtl
name=Clear
texture.transmission_filter=models/textures/transmittance.texture
texture.transmission_filter.source_channel=alpha
custom.transmission_filter.vec3=0.1,0.2,0.3
"
    .to_vec();
    let transmission_source = texture_bytes(1, 1, 93);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/transmittance_alias.mtl", material_source);
    io.insert(transmission_path.path(), transmission_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let transmission_id = database.import_asset_path(&transmission_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Clear"]);
    assert_eq!(material_metadata.dependencies, vec![transmission_id]);
    assert_eq!(
        (material_metadata.importer.as_deref()),
        Some("ModelImporter")
    );
    assert_eq!(material_metadata.importer_version, 111);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![transmission_id, mesh_id, material_id]
    );

    database
        .cook_asset(transmission_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "transmittance_alias_model",
            vec![mesh_id, material_id, transmission_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([transmission_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(
        reader.read_path(&transmission_path).unwrap(),
        transmission_source
    );

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        vec![transmission_id]
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "transmission_filter");
    assert_eq!(material.textures[0].texture.id(), transmission_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Alpha)
    );
    assert_eq!(
        material.properties.custom.get("transmission_filter"),
        Some(&MaterialPropertyValue::Vec3([0.1, 0.2, 0.3]))
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_maps_obj_alpha_texture_to_alpha_mode() {
    let config = database_config("builtin_model_obj_alpha_texture_alpha_mode");
    let model_path = AssetPath::parse("models/alpha_map.obj");
    let mesh_path = AssetPath::parse("models/alpha_map.Panel.mesh");
    let material_path = AssetPath::parse("models/alpha_map.Material_Cutout.material");
    let alpha_path = AssetPath::parse("models/textures/panel_alpha.texture");
    let model_source = b"mtllib alpha_map.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Cutout
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Cutout
map_d textures/panel_alpha.texture
"
    .to_vec();
    let expected_material = b"# mtllib alpha_map.mtl
name=Cutout
texture.alpha=models/textures/panel_alpha.texture
alpha_mode=blend
"
    .to_vec();
    let alpha_source = texture_bytes(1, 1, 88);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/alpha_map.mtl", material_source);
    io.insert(alpha_path.path(), alpha_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let alpha_id = database.import_asset_path(&alpha_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Cutout"]);
    assert_eq!(material_metadata.dependencies, vec![alpha_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![alpha_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![alpha_id]
    );

    database
        .cook_asset(alpha_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "alpha_map_model",
            vec![mesh_id, material_id, alpha_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([alpha_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(alpha_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&alpha_path).unwrap(), alpha_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        vec![alpha_id]
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.render_state.alpha_mode, AlphaMode::Blend);
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "alpha");
    assert_eq!(material.textures[0].texture.id(), alpha_id);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_maps_obj_transparency_texture_alias_to_alpha_mode() {
    let config = database_config("builtin_model_obj_transparency_texture_alpha_mode");
    let model_path = AssetPath::parse("models/transparency_map.obj");
    let mesh_path = AssetPath::parse("models/transparency_map.Panel.mesh");
    let material_path = AssetPath::parse("models/transparency_map.Material_Cutout.material");
    let alpha_path = AssetPath::parse("models/textures/panel_alpha.texture");
    let model_source = b"mtllib transparency_map.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Cutout
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Cutout
map_Tr textures/panel_alpha.texture
"
    .to_vec();
    let expected_material = b"# mtllib transparency_map.mtl
name=Cutout
texture.alpha=models/textures/panel_alpha.texture
alpha_mode=blend
"
    .to_vec();
    let alpha_source = texture_bytes(1, 1, 89);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/transparency_map.mtl", material_source);
    io.insert(alpha_path.path(), alpha_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let alpha_id = database.import_asset_path(&alpha_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Cutout"]);
    assert_eq!(material_metadata.dependencies, vec![alpha_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![alpha_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![alpha_id]
    );

    database
        .cook_asset(alpha_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "transparency_map_model",
            vec![mesh_id, material_id, alpha_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([alpha_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(alpha_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&alpha_path).unwrap(), alpha_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        vec![alpha_id]
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.render_state.alpha_mode, AlphaMode::Blend);
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "alpha");
    assert_eq!(material.textures[0].texture.id(), alpha_id);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_maps_obj_common_alpha_texture_aliases_to_alpha_mode() {
    for (directive, stem, texel) in [
        ("map_opacity", "opacity_alias", 90),
        ("map_alpha", "alpha_alias", 91),
        ("map_transparency", "transparency_alias", 92),
    ] {
        let config = database_config(&format!("builtin_model_obj_{stem}"));
        let model_path = AssetPath::parse(&format!("models/{stem}.obj"));
        let mesh_path = AssetPath::parse(&format!("models/{stem}.Panel.mesh"));
        let material_path = AssetPath::parse(&format!("models/{stem}.Material_Cutout.material"));
        let alpha_path = AssetPath::parse(&format!("models/textures/{stem}_alpha.texture"));
        let material_library = format!("{stem}.mtl");
        let model_source = format!(
            "mtllib {material_library}\n\
o Panel\n\
v 0 0 0\n\
v 1 0 0\n\
v 0 1 0\n\
usemtl Cutout\n\
f 1 2 3\n"
        )
        .into_bytes();
        let material_source =
            format!("newmtl Cutout\n{directive} textures/{stem}_alpha.texture\n").into_bytes();
        let expected_material = format!(
            "# mtllib {material_library}\n\
name=Cutout\n\
texture.alpha=models/textures/{stem}_alpha.texture\n\
alpha_mode=blend\n"
        )
        .into_bytes();
        let alpha_source = texture_bytes(1, 1, texel);
        let mut io = MemoryAssetIo::new();
        io.insert(model_path.path(), model_source);
        io.insert(format!("models/{material_library}"), material_source);
        io.insert(alpha_path.path(), alpha_source.clone());
        let mut database = AssetDatabase::new(config.clone());
        database.set_io(io);
        database.register_builtin_importers();
        database.register_builtin_cookers();

        let alpha_id = database.import_asset_path(&alpha_path).unwrap();
        let model_id = database.import_asset_path(&model_path).unwrap();
        let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
        let material_metadata = database
            .registry()
            .metadata_by_path(&material_path)
            .unwrap();
        let material_id = material_metadata.id;

        assert_eq!(material_metadata.labels, vec!["Material/Cutout"]);
        assert_eq!(material_metadata.dependencies, vec![alpha_id]);
        assert_eq!(
            fs::read(config.imported_root.join(material_path.path())).unwrap(),
            expected_material
        );
        assert_eq!(
            database.registry().get(model_id).unwrap().dependencies,
            vec![alpha_id, mesh_id, material_id]
        );

        database
            .cook_asset(alpha_id, TargetPlatform::Windows)
            .unwrap();
        database
            .cook_asset(mesh_id, TargetPlatform::Windows)
            .unwrap();
        database
            .cook_asset(material_id, TargetPlatform::Windows)
            .unwrap();
        let bundle = database
            .build_bundle(&AssetDatabaseBundleBuild::new(
                format!("{stem}_model"),
                vec![mesh_id, material_id, alpha_id],
            ))
            .unwrap();
        let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
        assert_eq!(
            reader.manifest().dependencies(mesh_id),
            Some([material_id].as_slice())
        );
        assert_eq!(
            reader.manifest().dependencies(material_id),
            Some([alpha_id].as_slice())
        );
        assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
        assert_eq!(reader.read_path(&alpha_path).unwrap(), alpha_source);

        let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
        let mut server = AssetServer::new(AssetServerConfig::default());
        server.set_io(bundle_io);
        server.register_builtin_loaders();
        let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
        let group = server.preload_bundle(&mounted);
        for _ in 0..8 {
            server.update_loading();
            finish_uploads(&mut server);
            if server.group_state(&group) == AssetLoadState::Ready {
                break;
            }
        }

        assert_eq!(server.group_state(&group), AssetLoadState::Ready);
        assert_eq!(
            server.dependency_graph().direct_dependencies(material_id),
            vec![alpha_id]
        );
        let material = server.get_by_id::<Material>(material_id).unwrap();
        assert_eq!(material.render_state.alpha_mode, AlphaMode::Blend);
        assert_eq!(material.textures.len(), 1);
        assert_eq!(material.textures[0].name, "alpha");
        assert_eq!(material.textures[0].texture.id(), alpha_id);
    }
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_alpha_source_channel() {
    let config = database_config("builtin_model_obj_alpha_source_channel");
    let model_path = AssetPath::parse("models/alpha_source.obj");
    let mesh_path = AssetPath::parse("models/alpha_source.Panel.mesh");
    let material_path = AssetPath::parse("models/alpha_source.Material_Cutout.material");
    let alpha_path = AssetPath::parse("models/textures/panel_alpha.texture");
    let model_source = b"mtllib alpha_source.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Cutout
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Cutout
map_d -imfchan A textures/panel_alpha.texture
"
    .to_vec();
    let expected_material = b"# mtllib alpha_source.mtl
name=Cutout
texture.alpha=models/textures/panel_alpha.texture
texture.alpha.source_channel=alpha
alpha_mode=blend
"
    .to_vec();
    let alpha_source = texture_bytes(1, 1, 93);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/alpha_source.mtl", material_source);
    io.insert(alpha_path.path(), alpha_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let alpha_id = database.import_asset_path(&alpha_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Cutout"]);
    assert_eq!(material_metadata.dependencies, vec![alpha_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![alpha_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![alpha_id]
    );

    database
        .cook_asset(alpha_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "alpha_source_model",
            vec![mesh_id, material_id, alpha_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([alpha_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(alpha_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&alpha_path).unwrap(), alpha_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        vec![alpha_id]
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.render_state.alpha_mode, AlphaMode::Blend);
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "alpha");
    assert_eq!(material.textures[0].texture.id(), alpha_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Alpha)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_transparency_source_channel() {
    let config = database_config("builtin_model_obj_transparency_source_channel");
    let model_path = AssetPath::parse("models/transparency_source.obj");
    let mesh_path = AssetPath::parse("models/transparency_source.Panel.mesh");
    let material_path = AssetPath::parse("models/transparency_source.Material_Cutout.material");
    let alpha_path = AssetPath::parse("models/textures/panel_alpha.texture");
    let model_source = b"mtllib transparency_source.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Cutout
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Cutout
map_Tr -imfchan blue textures/panel_alpha.texture
"
    .to_vec();
    let expected_material = b"# mtllib transparency_source.mtl
name=Cutout
texture.alpha=models/textures/panel_alpha.texture
texture.alpha.source_channel=blue
alpha_mode=blend
"
    .to_vec();
    let alpha_source = texture_bytes(1, 1, 94);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/transparency_source.mtl", material_source);
    io.insert(alpha_path.path(), alpha_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let alpha_id = database.import_asset_path(&alpha_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Cutout"]);
    assert_eq!(material_metadata.dependencies, vec![alpha_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![alpha_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![alpha_id]
    );

    database
        .cook_asset(alpha_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "transparency_source_model",
            vec![mesh_id, material_id, alpha_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([alpha_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(alpha_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&alpha_path).unwrap(), alpha_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        vec![alpha_id]
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.render_state.alpha_mode, AlphaMode::Blend);
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "alpha");
    assert_eq!(material.textures[0].texture.id(), alpha_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Blue)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_occlusion_and_emissive_source_channels() {
    let config = database_config("builtin_model_obj_occlusion_emissive_source_channels");
    let model_path = AssetPath::parse("models/light_source.obj");
    let mesh_path = AssetPath::parse("models/light_source.Panel.mesh");
    let material_path = AssetPath::parse("models/light_source.Material_Lit.material");
    let occlusion_path = AssetPath::parse("models/textures/panel_ao.texture");
    let emissive_path = AssetPath::parse("models/textures/panel_emissive.texture");
    let model_source = b"mtllib light_source.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Lit
f 1 2 3
"
    .to_vec();
let material_source = b"newmtl Lit
map_Ka -imfchan green -colorspace Non-Color textures/panel_ao.texture
map_Ke -imfchan red -colorspace Non-Color textures/panel_emissive.texture
"
    .to_vec();
    let expected_material = b"# mtllib light_source.mtl
name=Lit
texture.occlusion=models/textures/panel_ao.texture
texture.occlusion.source_channel=green
texture.occlusion.color_space=non_color
texture.emissive=models/textures/panel_emissive.texture
texture.emissive.source_channel=red
texture.emissive.color_space=non_color
"
    .to_vec();
    let occlusion_source = texture_bytes(1, 1, 95);
    let emissive_source = texture_bytes(1, 1, 96);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/light_source.mtl", material_source);
    io.insert(occlusion_path.path(), occlusion_source.clone());
    io.insert(emissive_path.path(), emissive_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let occlusion_id = database.import_asset_path(&occlusion_path).unwrap();
    let emissive_id = database.import_asset_path(&emissive_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Lit"]);
    assert_eq!(
        material_metadata.dependencies,
        vec![occlusion_id, emissive_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![occlusion_id, emissive_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![occlusion_id, emissive_id]
    );

    database
        .cook_asset(occlusion_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(emissive_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "light_source_model",
            vec![mesh_id, material_id, occlusion_id, emissive_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([occlusion_id, emissive_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&occlusion_path).unwrap(), occlusion_source);
    assert_eq!(reader.read_path(&emissive_path).unwrap(), emissive_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        vec![occlusion_id, emissive_id]
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 2);
    assert_eq!(material.textures[0].name, "occlusion");
    assert_eq!(material.textures[0].texture.id(), occlusion_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Green)
    );
    assert_eq!(
        material.textures[0].options.color_space,
        Some(MaterialTextureColorSpace::NonColor)
    );
    assert_eq!(material.textures[1].name, "emissive");
    assert_eq!(material.textures[1].texture.id(), emissive_id);
    assert_eq!(
        material.textures[1].options.source_channel,
        Some(MaterialTextureChannel::Red)
    );
    assert_eq!(
        material.textures[1].options.color_space,
        Some(MaterialTextureColorSpace::NonColor)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_ambient_occlusion_texture_aliases() {
    let config = database_config("builtin_model_obj_ambient_occlusion_texture_aliases");
    let model_path = AssetPath::parse("models/ao_aliases.obj");
    let mesh_paths = [
        AssetPath::parse("models/ao_aliases.RawPanel.mesh"),
        AssetPath::parse("models/ao_aliases.NamedPanel.mesh"),
        AssetPath::parse("models/ao_aliases.LongPanel.mesh"),
    ];
    let material_paths = [
        AssetPath::parse("models/ao_aliases.Material_RawAo.material"),
        AssetPath::parse("models/ao_aliases.Material_NamedAo.material"),
        AssetPath::parse("models/ao_aliases.Material_LongAo.material"),
    ];
    let texture_paths = [
        AssetPath::parse("models/textures/raw_ao.texture"),
        AssetPath::parse("models/textures/named_ao.texture"),
        AssetPath::parse("models/textures/long_ao.texture"),
    ];
    let material_names = ["RawAo", "NamedAo", "LongAo"];
    let expected_channels = [
        MaterialTextureChannel::Red,
        MaterialTextureChannel::Blue,
        MaterialTextureChannel::Green,
    ];
    let model_source = b"mtllib ao_aliases.mtl
o RawPanel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl RawAo
f 1 2 3
o NamedPanel
v 0 0 1
v 1 0 1
v 0 1 1
usemtl NamedAo
f 4 5 6
o LongPanel
v 0 0 2
v 1 0 2
v 0 1 2
usemtl LongAo
f 7 8 9
"
    .to_vec();
    let material_source = b"newmtl RawAo
map_AO -imfchan r -colorspace Non-Color textures/raw_ao.texture
newmtl NamedAo
map_Occlusion -imfchan blue -colorspace Non-Color textures/named_ao.texture
newmtl LongAo
map_ambient_occlusion -imfchan G -colorspace Non-Color textures/long_ao.texture
"
    .to_vec();
    let expected_materials = [
        b"# mtllib ao_aliases.mtl
name=RawAo
texture.occlusion=models/textures/raw_ao.texture
texture.occlusion.source_channel=red
texture.occlusion.color_space=non_color
"
        .to_vec(),
        b"# mtllib ao_aliases.mtl
name=NamedAo
texture.occlusion=models/textures/named_ao.texture
texture.occlusion.source_channel=blue
texture.occlusion.color_space=non_color
"
        .to_vec(),
        b"# mtllib ao_aliases.mtl
name=LongAo
texture.occlusion=models/textures/long_ao.texture
texture.occlusion.source_channel=green
texture.occlusion.color_space=non_color
"
        .to_vec(),
    ];
    let texture_sources = [
        texture_bytes(1, 1, 48),
        texture_bytes(1, 1, 49),
        texture_bytes(1, 1, 50),
    ];
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/ao_aliases.mtl", material_source);
    for (path, source) in texture_paths.iter().zip(texture_sources.iter()) {
        io.insert(path.path(), source.clone());
    }
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let texture_ids = texture_paths
        .iter()
        .map(|path| database.import_asset_path(path).unwrap())
        .collect::<Vec<_>>();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_ids = mesh_paths
        .iter()
        .map(|path| database.registry().metadata_by_path(path).unwrap().id)
        .collect::<Vec<_>>();
    let material_ids = material_paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let metadata = database.registry().metadata_by_path(path).unwrap();
            assert_eq!(
                metadata.labels,
                vec![format!("Material/{}", material_names[index])]
            );
            assert_eq!(metadata.dependencies, vec![texture_ids[index]]);
            assert_eq!(
                fs::read(config.imported_root.join(path.path())).unwrap(),
                expected_materials[index]
            );
            metadata.id
        })
        .collect::<Vec<_>>();

    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert_eq!(
        model_dependencies.len(),
        texture_ids.len() + mesh_ids.len() + material_ids.len()
    );
    for id in texture_ids
        .iter()
        .chain(mesh_ids.iter())
        .chain(material_ids.iter())
    {
        assert!(model_dependencies.contains(id));
    }

    for id in texture_ids
        .iter()
        .chain(mesh_ids.iter())
        .chain(material_ids.iter())
    {
        database.cook_asset(*id, TargetPlatform::Windows).unwrap();
    }
    let mut bundle_ids = Vec::new();
    bundle_ids.extend(mesh_ids.iter().copied());
    bundle_ids.extend(material_ids.iter().copied());
    bundle_ids.extend(texture_ids.iter().copied());
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "ambient_occlusion_texture_aliases",
            bundle_ids,
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    for index in 0..material_ids.len() {
        assert_eq!(
            reader.manifest().dependencies(mesh_ids[index]),
            Some([material_ids[index]].as_slice())
        );
        assert_eq!(
            reader.manifest().dependencies(material_ids[index]),
            Some([texture_ids[index]].as_slice())
        );
        assert_eq!(
            reader.manifest().dependencies(texture_ids[index]),
            Some([].as_slice())
        );
        assert_eq!(
            reader.read_path(&material_paths[index]).unwrap(),
            expected_materials[index]
        );
        assert_eq!(
            reader.read_path(&texture_paths[index]).unwrap(),
            texture_sources[index]
        );
    }

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..12 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    for index in 0..material_ids.len() {
        assert_eq!(
            server
                .dependency_graph()
                .direct_dependencies(material_ids[index]),
            vec![texture_ids[index]]
        );
        let material = server.get_by_id::<Material>(material_ids[index]).unwrap();
        assert_eq!(material.textures.len(), 1);
        assert_eq!(material.textures[0].name, "occlusion");
        assert_eq!(material.textures[0].texture.id(), texture_ids[index]);
        assert_eq!(
            material.textures[0].options.source_channel,
            Some(expected_channels[index])
        );
        assert_eq!(
            material.textures[0].options.color_space,
            Some(MaterialTextureColorSpace::NonColor)
        );
    }
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_pbr_material_extensions() {
    let config = database_config("builtin_model_obj_pbr_material_extensions");
    let model_path = AssetPath::parse("models/pbr.obj");
    let mesh_path = AssetPath::parse("models/pbr.Panel.mesh");
    let material_path = AssetPath::parse("models/pbr.Material_Coat.material");
    let texture_paths = [
        AssetPath::parse("models/textures/pbr_sheen.texture"),
        AssetPath::parse("models/textures/pbr_clearcoat.texture"),
        AssetPath::parse("models/textures/pbr_clearcoat_roughness.texture"),
        AssetPath::parse("models/textures/pbr_anisotropy.texture"),
        AssetPath::parse("models/textures/pbr_anisotropy_rotation.texture"),
    ];
    let texture_sources = [
        texture_bytes(1, 1, 31),
        texture_bytes(1, 1, 32),
        texture_bytes(1, 1, 33),
        texture_bytes(1, 1, 34),
        texture_bytes(1, 1, 35),
    ];
    let model_source = b"mtllib pbr.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Coat
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Coat
PS 0.25
pc 0.6
PCR 0.15
ANISO 0.8
ANISOR 0.1
map_Ps textures/pbr_sheen.texture
map_Pc textures/pbr_clearcoat.texture
map_Pcr textures/pbr_clearcoat_roughness.texture
map_aniso textures/pbr_anisotropy.texture
map_anisor textures/pbr_anisotropy_rotation.texture
"
    .to_vec();
    let expected_material = b"# mtllib pbr.mtl
name=Coat
texture.sheen=models/textures/pbr_sheen.texture
texture.clearcoat=models/textures/pbr_clearcoat.texture
texture.clearcoat_roughness=models/textures/pbr_clearcoat_roughness.texture
texture.anisotropy=models/textures/pbr_anisotropy.texture
texture.anisotropy_rotation=models/textures/pbr_anisotropy_rotation.texture
custom.sheen.float=0.25
custom.clearcoat.float=0.6
custom.clearcoat_roughness.float=0.15
custom.anisotropy.float=0.8
custom.anisotropy_rotation.float=0.1
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/pbr.mtl", material_source);
    for (path, source) in texture_paths.iter().zip(texture_sources.iter()) {
        io.insert(path.path(), source.clone());
    }
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let texture_ids = texture_paths
        .iter()
        .map(|path| database.import_asset_path(path).unwrap())
        .collect::<Vec<_>>();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Coat"]);
    assert_eq!(material_metadata.dependencies, texture_ids);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    let mut expected_root_dependencies = texture_ids.clone();
    expected_root_dependencies.extend([mesh_id, material_id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        expected_root_dependencies
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        texture_ids
    );

    for texture_id in &texture_ids {
        database
            .cook_asset(*texture_id, TargetPlatform::Windows)
            .unwrap();
    }
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let mut bundle_assets = vec![mesh_id, material_id];
    bundle_assets.extend(texture_ids.iter().copied());
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "pbr_material_model",
            bundle_assets,
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some(texture_ids.as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    for (path, source) in texture_paths.iter().zip(texture_sources.iter()) {
        assert_eq!(reader.read_path(path).unwrap(), source.clone());
    }

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        texture_ids
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(
        material.properties.custom.get("sheen"),
        Some(&MaterialPropertyValue::Float(0.25))
    );
    assert_eq!(
        material.properties.custom.get("clearcoat"),
        Some(&MaterialPropertyValue::Float(0.6))
    );
    assert_eq!(
        material.properties.custom.get("clearcoat_roughness"),
        Some(&MaterialPropertyValue::Float(0.15))
    );
    assert_eq!(
        material.properties.custom.get("anisotropy"),
        Some(&MaterialPropertyValue::Float(0.8))
    );
    assert_eq!(
        material.properties.custom.get("anisotropy_rotation"),
        Some(&MaterialPropertyValue::Float(0.1))
    );
    let texture_names = [
        "sheen",
        "clearcoat",
        "clearcoat_roughness",
        "anisotropy",
        "anisotropy_rotation",
    ];
    assert_eq!(material.textures.len(), texture_names.len());
    for (index, name) in texture_names.iter().enumerate() {
        assert_eq!(material.textures[index].name, *name);
        assert_eq!(material.textures[index].texture.id(), texture_ids[index]);
    }
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_maps_obj_common_pbr_texture_scalar_aliases() {
    let config = database_config("builtin_model_obj_pbr_texture_scalar_aliases");
    let model_path = AssetPath::parse("models/pbr_texture_scalar_aliases.obj");
    let mesh_path = AssetPath::parse("models/pbr_texture_scalar_aliases.Panel.mesh");
    let material_path =
        AssetPath::parse("models/pbr_texture_scalar_aliases.Material_AliasMaps.material");
    let texture_paths = [
        AssetPath::parse("models/textures/alias_specular.texture"),
        AssetPath::parse("models/textures/alias_ambient.texture"),
        AssetPath::parse("models/textures/alias_emissive.texture"),
        AssetPath::parse("models/textures/alias_transmission.texture"),
        AssetPath::parse("models/textures/alias_ior.texture"),
        AssetPath::parse("models/textures/alias_clearcoat.texture"),
        AssetPath::parse("models/textures/alias_clearcoat_roughness.texture"),
        AssetPath::parse("models/textures/alias_anisotropy_rotation.texture"),
    ];
    let texture_sources = [
        texture_bytes(1, 1, 141),
        texture_bytes(1, 1, 142),
        texture_bytes(1, 1, 143),
        texture_bytes(1, 1, 144),
        texture_bytes(1, 1, 145),
        texture_bytes(1, 1, 146),
        texture_bytes(1, 1, 147),
        texture_bytes(1, 1, 148),
    ];
    let model_source = b"mtllib pbr_texture_scalar_aliases.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl AliasMaps
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl AliasMaps
map_specular_color -imfchan blue textures/alias_specular.texture
map_ambient_color textures/alias_ambient.texture
map_emissive_color textures/alias_emissive.texture
map_transmission_color textures/alias_transmission.texture
map_ior textures/alias_ior.texture
map_clear_coat textures/alias_clearcoat.texture
map_clear_coat_roughness textures/alias_clearcoat_roughness.texture
map_anisotropyrotation textures/alias_anisotropy_rotation.texture
"
    .to_vec();
    let expected_material = b"# mtllib pbr_texture_scalar_aliases.mtl
name=AliasMaps
texture.specular=models/textures/alias_specular.texture
texture.specular.source_channel=blue
texture.occlusion=models/textures/alias_ambient.texture
texture.emissive=models/textures/alias_emissive.texture
texture.transmission_filter=models/textures/alias_transmission.texture
texture.index_of_refraction=models/textures/alias_ior.texture
texture.clearcoat=models/textures/alias_clearcoat.texture
texture.clearcoat_roughness=models/textures/alias_clearcoat_roughness.texture
texture.anisotropy_rotation=models/textures/alias_anisotropy_rotation.texture
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/pbr_texture_scalar_aliases.mtl", material_source);
    for (path, source) in texture_paths.iter().zip(texture_sources.iter()) {
        io.insert(path.path(), source.clone());
    }
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let texture_ids = texture_paths
        .iter()
        .map(|path| database.import_asset_path(path).unwrap())
        .collect::<Vec<_>>();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/AliasMaps"]);
    assert_eq!(material_metadata.dependencies, texture_ids);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    let mut expected_root_dependencies = texture_ids.clone();
    expected_root_dependencies.extend([mesh_id, material_id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        expected_root_dependencies
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        texture_ids
    );

    for texture_id in &texture_ids {
        database
            .cook_asset(*texture_id, TargetPlatform::Windows)
            .unwrap();
    }
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let mut bundle_assets = vec![mesh_id, material_id];
    bundle_assets.extend(texture_ids.iter().copied());
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "pbr_texture_scalar_aliases",
            bundle_assets,
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some(texture_ids.as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    for (path, source) in texture_paths.iter().zip(texture_sources.iter()) {
        assert_eq!(reader.read_path(path).unwrap(), source.clone());
    }

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        texture_ids
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    let texture_names = [
        "specular",
        "occlusion",
        "emissive",
        "transmission_filter",
        "index_of_refraction",
        "clearcoat",
        "clearcoat_roughness",
        "anisotropy_rotation",
    ];
    assert_eq!(material.textures.len(), texture_names.len());
    for (index, name) in texture_names.iter().enumerate() {
        assert_eq!(material.textures[index].name, *name);
        assert_eq!(material.textures[index].texture.id(), texture_ids[index]);
    }
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Blue)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_maps_obj_common_pbr_scalar_aliases() {
    let config = database_config("builtin_model_obj_pbr_scalar_aliases");
    let model_path = AssetPath::parse("models/pbr_scalar_aliases.obj");
    let mesh_path = AssetPath::parse("models/pbr_scalar_aliases.Panel.mesh");
    let material_path =
        AssetPath::parse("models/pbr_scalar_aliases.Material_AliasSurface.material");
    let model_source = b"mtllib pbr_scalar_aliases.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl AliasSurface
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl AliasSurface
base_color 0.1 0.2 0.3
opacity 0.7
ambient_color 0.01 0.02 0.03
specular_color 0.4 0.5 0.6
emissive_color 0.05 0.06 0.07
transmission_color 0.2 0.25 0.3
metalness 0.4
roughness 0.35
clear_coat 0.55
clear_coat_roughness 0.22
anisotropy 0.66
anisotropyrotation 0.12
"
    .to_vec();
    let expected_material = b"# mtllib pbr_scalar_aliases.mtl
name=AliasSurface
custom.ambient_color.vec3=0.01,0.02,0.03
custom.specular_color.vec3=0.4,0.5,0.6
custom.transmission_filter.vec3=0.2,0.25,0.3
custom.clearcoat.float=0.55
custom.clearcoat_roughness.float=0.22
custom.anisotropy.float=0.66
custom.anisotropy_rotation.float=0.12
base_color=0.1,0.2,0.3,0.7
alpha_mode=blend
metallic=0.4
roughness=0.35
emissive=0.05,0.06,0.07
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/pbr_scalar_aliases.mtl", material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/AliasSurface"]);
    assert!(material_metadata.dependencies.is_empty());
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, material_id]
    );

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "pbr_scalar_aliases",
            vec![mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        Vec::<AssetId>::new()
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.properties.base_color, [0.1, 0.2, 0.3, 0.7]);
    assert_eq!(material.render_state.alpha_mode, AlphaMode::Blend);
    assert_eq!(material.properties.metallic, 0.4);
    assert_eq!(material.properties.roughness, 0.35);
    assert_eq!(material.properties.emissive, [0.05, 0.06, 0.07]);
    assert_eq!(
        material.properties.custom.get("ambient_color"),
        Some(&MaterialPropertyValue::Vec3([0.01, 0.02, 0.03]))
    );
    assert_eq!(
        material.properties.custom.get("specular_color"),
        Some(&MaterialPropertyValue::Vec3([0.4, 0.5, 0.6]))
    );
    assert_eq!(
        material.properties.custom.get("transmission_filter"),
        Some(&MaterialPropertyValue::Vec3([0.2, 0.25, 0.3]))
    );
    assert_eq!(
        material.properties.custom.get("clearcoat"),
        Some(&MaterialPropertyValue::Float(0.55))
    );
    assert_eq!(
        material.properties.custom.get("clearcoat_roughness"),
        Some(&MaterialPropertyValue::Float(0.22))
    );
    assert_eq!(
        material.properties.custom.get("anisotropy"),
        Some(&MaterialPropertyValue::Float(0.66))
    );
    assert_eq!(
        material.properties.custom.get("anisotropy_rotation"),
        Some(&MaterialPropertyValue::Float(0.12))
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_pbr_texture_aliases() {
    let config = database_config("builtin_model_obj_pbr_texture_aliases");
    let model_path = AssetPath::parse("models/pbr_aliases.obj");
    let mesh_path = AssetPath::parse("models/pbr_aliases.Panel.mesh");
    let material_path = AssetPath::parse("models/pbr_aliases.Material_Coat.material");
    let texture_paths = [
        AssetPath::parse("models/textures/pbr_sheen.texture"),
        AssetPath::parse("models/textures/pbr_clearcoat.texture"),
        AssetPath::parse("models/textures/pbr_clearcoat_roughness.texture"),
        AssetPath::parse("models/textures/pbr_anisotropy.texture"),
        AssetPath::parse("models/textures/pbr_anisotropy_rotation.texture"),
    ];
    let texture_sources = [
        texture_bytes(1, 1, 41),
        texture_bytes(1, 1, 42),
        texture_bytes(1, 1, 43),
        texture_bytes(1, 1, 44),
        texture_bytes(1, 1, 45),
    ];
    let model_source = b"mtllib pbr_aliases.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Coat
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Coat
Ps 0.25
Pc 0.6
Pcr 0.15
aniso 0.8
anisor 0.1
MAP_SHEEN -imfchan red textures/pbr_sheen.texture
Map_Clearcoat -imfchan green textures/pbr_clearcoat.texture
MAP_CLEARCOAT_ROUGHNESS -imfchan blue textures/pbr_clearcoat_roughness.texture
Map_Anisotropy -imfchan matte textures/pbr_anisotropy.texture
MAP_ANISOTROPY_ROTATION -imfchan luminance textures/pbr_anisotropy_rotation.texture
"
    .to_vec();
    let expected_material = b"# mtllib pbr_aliases.mtl
name=Coat
texture.sheen=models/textures/pbr_sheen.texture
texture.sheen.source_channel=red
texture.clearcoat=models/textures/pbr_clearcoat.texture
texture.clearcoat.source_channel=green
texture.clearcoat_roughness=models/textures/pbr_clearcoat_roughness.texture
texture.clearcoat_roughness.source_channel=blue
texture.anisotropy=models/textures/pbr_anisotropy.texture
texture.anisotropy.source_channel=matte
texture.anisotropy_rotation=models/textures/pbr_anisotropy_rotation.texture
texture.anisotropy_rotation.source_channel=luminance
custom.sheen.float=0.25
custom.clearcoat.float=0.6
custom.clearcoat_roughness.float=0.15
custom.anisotropy.float=0.8
custom.anisotropy_rotation.float=0.1
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/pbr_aliases.mtl", material_source);
    for (path, source) in texture_paths.iter().zip(texture_sources.iter()) {
        io.insert(path.path(), source.clone());
    }
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let texture_ids = texture_paths
        .iter()
        .map(|path| database.import_asset_path(path).unwrap())
        .collect::<Vec<_>>();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Coat"]);
    assert_eq!(material_metadata.dependencies, texture_ids);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    let mut expected_root_dependencies = texture_ids.clone();
    expected_root_dependencies.extend([mesh_id, material_id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        expected_root_dependencies
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        texture_ids
    );

    for texture_id in &texture_ids {
        database
            .cook_asset(*texture_id, TargetPlatform::Windows)
            .unwrap();
    }
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new("pbr_texture_aliases", {
            let mut entries = texture_ids.clone();
            entries.extend([mesh_id, material_id]);
            entries
        }))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some(texture_ids.as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    for (path, source) in texture_paths.iter().zip(texture_sources.iter()) {
        assert_eq!(reader.read_path(path).unwrap(), *source);
    }

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        texture_ids
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    let texture_names = [
        "sheen",
        "clearcoat",
        "clearcoat_roughness",
        "anisotropy",
        "anisotropy_rotation",
    ];
    assert_eq!(material.textures.len(), texture_names.len());
    let expected_channels = [
        Some(MaterialTextureChannel::Red),
        Some(MaterialTextureChannel::Green),
        Some(MaterialTextureChannel::Blue),
        Some(MaterialTextureChannel::Matte),
        Some(MaterialTextureChannel::Luminance),
    ];
    for (index, name) in texture_names.iter().enumerate() {
        assert_eq!(material.textures[index].name, *name);
        assert_eq!(material.textures[index].texture.id(), texture_ids[index]);
        assert_eq!(
            material.textures[index].options.source_channel,
            expected_channels[index]
        );
    }
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_maps_obj_common_pbr_texture_aliases() {
    for (directive, stem, channel, texel) in [
        ("map_diffuse", "diffuse_alias", "albedo", 110),
        ("map_albedo", "albedo_alias", "albedo", 111),
        ("map_basecolor", "basecolor_alias", "albedo", 112),
        ("map_base_color", "base_color_alias", "albedo", 113),
        ("map_specular", "specular_alias", "specular", 114),
        ("map_emissive", "emissive_alias", "emissive", 115),
        ("map_emission", "emission_alias", "emissive", 116),
        ("map_roughness", "roughness_alias", "roughness", 117),
        ("map_metallic", "metallic_alias", "metallic", 118),
        ("map_metalness", "metalness_alias", "metallic", 119),
        ("map_normalgl", "normalgl_alias", "normal", 120),
        ("map_normaldx", "normaldx_alias", "normal", 121),
    ] {
        let config = database_config(&format!("builtin_model_obj_{stem}"));
        let model_path = AssetPath::parse(&format!("models/{stem}.obj"));
        let mesh_path = AssetPath::parse(&format!("models/{stem}.Panel.mesh"));
        let material_path = AssetPath::parse(&format!("models/{stem}.Material_Surface.material"));
        let texture_path = AssetPath::parse(&format!("models/textures/{stem}.texture"));
        let material_library = format!("{stem}.mtl");
        let model_source = format!(
            "mtllib {material_library}\n\
o Panel\n\
v 0 0 0\n\
v 1 0 0\n\
v 0 1 0\n\
usemtl Surface\n\
f 1 2 3\n"
        )
        .into_bytes();
        let material_source =
            format!("newmtl Surface\n{directive} textures/{stem}.texture\n").into_bytes();
        let expected_material = format!(
            "# mtllib {material_library}\n\
name=Surface\n\
texture.{channel}=models/textures/{stem}.texture\n"
        )
        .into_bytes();
        let texture_source = texture_bytes(1, 1, texel);
        let mut io = MemoryAssetIo::new();
        io.insert(model_path.path(), model_source);
        io.insert(format!("models/{material_library}"), material_source);
        io.insert(texture_path.path(), texture_source.clone());
        let mut database = AssetDatabase::new(config.clone());
        database.set_io(io);
        database.register_builtin_importers();
        database.register_builtin_cookers();

        let texture_id = database.import_asset_path(&texture_path).unwrap();
        let model_id = database.import_asset_path(&model_path).unwrap();
        let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
        let material_metadata = database
            .registry()
            .metadata_by_path(&material_path)
            .unwrap();
        let material_id = material_metadata.id;

        assert_eq!(material_metadata.labels, vec!["Material/Surface"]);
        assert_eq!(material_metadata.dependencies, vec![texture_id]);
        assert_eq!(
            fs::read(config.imported_root.join(material_path.path())).unwrap(),
            expected_material
        );
        assert_eq!(
            database.registry().get(model_id).unwrap().dependencies,
            vec![texture_id, mesh_id, material_id]
        );

        database
            .cook_asset(texture_id, TargetPlatform::Windows)
            .unwrap();
        database
            .cook_asset(mesh_id, TargetPlatform::Windows)
            .unwrap();
        database
            .cook_asset(material_id, TargetPlatform::Windows)
            .unwrap();
        let bundle = database
            .build_bundle(&AssetDatabaseBundleBuild::new(
                format!("{stem}_model"),
                vec![mesh_id, material_id, texture_id],
            ))
            .unwrap();
        let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
        assert_eq!(
            reader.manifest().dependencies(mesh_id),
            Some([material_id].as_slice())
        );
        assert_eq!(
            reader.manifest().dependencies(material_id),
            Some([texture_id].as_slice())
        );
        assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
        assert_eq!(reader.read_path(&texture_path).unwrap(), texture_source);

        let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
        let mut server = AssetServer::new(AssetServerConfig::default());
        server.set_io(bundle_io);
        server.register_builtin_loaders();
        let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
        let group = server.preload_bundle(&mounted);
        for _ in 0..8 {
            server.update_loading();
            finish_uploads(&mut server);
            if server.group_state(&group) == AssetLoadState::Ready {
                break;
            }
        }

        assert_eq!(server.group_state(&group), AssetLoadState::Ready);
        assert_eq!(
            server.dependency_graph().direct_dependencies(material_id),
            vec![texture_id]
        );
        let material = server.get_by_id::<Material>(material_id).unwrap();
        assert_eq!(material.textures.len(), 1);
        assert_eq!(material.textures[0].name, channel);
        assert_eq!(material.textures[0].texture.id(), texture_id);
    }
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_pbr_packed_texture_maps() {
    let config = database_config("builtin_model_obj_pbr_packed_texture_maps");
    let model_path = AssetPath::parse("models/packed_pbr.obj");
    let mesh_path = AssetPath::parse("models/packed_pbr.Panel.mesh");
    let material_path = AssetPath::parse("models/packed_pbr.Material_Coat.material");
    let rma_path = AssetPath::parse("models/textures/packed_rma.texture");
    let orm_path = AssetPath::parse("models/textures/packed_orm.texture");
    let model_source = b"mtllib packed_pbr.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Coat
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Coat
map_RMA -imfchan red -colorspace Non-Color textures/packed_rma.texture
map_ORM -imfchan green -clamp on textures/packed_orm.texture
"
    .to_vec();
    let expected_material = b"# mtllib packed_pbr.mtl
name=Coat
texture.rma=models/textures/packed_rma.texture
texture.rma.source_channel=red
texture.rma.color_space=non_color
texture.orm=models/textures/packed_orm.texture
texture.orm.sampler.address=clamp_to_edge
texture.orm.source_channel=green
"
    .to_vec();
    let rma_source = texture_bytes(1, 1, 46);
    let orm_source = texture_bytes(1, 1, 47);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/packed_pbr.mtl", material_source);
    io.insert(rma_path.path(), rma_source.clone());
    io.insert(orm_path.path(), orm_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let rma_id = database.import_asset_path(&rma_path).unwrap();
    let orm_id = database.import_asset_path(&orm_path).unwrap();
    let texture_ids = vec![rma_id, orm_id];
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Coat"]);
    assert_eq!(material_metadata.dependencies, texture_ids);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![rma_id, orm_id, mesh_id, material_id]
    );

    for texture_id in [rma_id, orm_id] {
        database
            .cook_asset(texture_id, TargetPlatform::Windows)
            .unwrap();
    }
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "pbr_packed_texture_maps",
            vec![mesh_id, material_id, rma_id, orm_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([rma_id, orm_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&rma_path).unwrap(), rma_source);
    assert_eq!(reader.read_path(&orm_path).unwrap(), orm_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        texture_ids
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 2);
    assert_eq!(material.textures[0].name, "rma");
    assert_eq!(material.textures[0].texture.id(), rma_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Red)
    );
    assert_eq!(
        material.textures[0].options.color_space,
        Some(MaterialTextureColorSpace::NonColor)
    );
    assert_eq!(material.textures[1].name, "orm");
    assert_eq!(material.textures[1].texture.id(), orm_id);
    assert_eq!(
        material.textures[1].sampler.address,
        AddressMode::ClampToEdge
    );
    assert_eq!(
        material.textures[1].options.source_channel,
        Some(MaterialTextureChannel::Green)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_pbr_packed_texture_aliases() {
    let config = database_config("builtin_model_obj_pbr_packed_texture_aliases");
    let model_path = AssetPath::parse("models/packed_aliases.obj");
    let mesh_paths = [
        AssetPath::parse("models/packed_aliases.ShortPanel.mesh"),
        AssetPath::parse("models/packed_aliases.LongPanel.mesh"),
    ];
    let material_paths = [
        AssetPath::parse("models/packed_aliases.Material_ShortPacked.material"),
        AssetPath::parse("models/packed_aliases.Material_LongPacked.material"),
    ];
    let texture_paths = [
        AssetPath::parse("models/textures/packed_mr.texture"),
        AssetPath::parse("models/textures/packed_mra.texture"),
        AssetPath::parse("models/textures/packed_metallicroughness.texture"),
        AssetPath::parse("models/textures/packed_arm.texture"),
    ];
    let model_source = b"mtllib packed_aliases.mtl
o ShortPanel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl ShortPacked
f 1 2 3
o LongPanel
v 0 0 1
v 1 0 1
v 0 1 1
usemtl LongPacked
f 4 5 6
"
    .to_vec();
    let material_source = b"newmtl ShortPacked
map_MR -imfchan green -colorspace Non-Color textures/packed_mr.texture
map_MRA -imfchan red textures/packed_mra.texture
newmtl LongPacked
map_metallicroughness -imfchan blue textures/packed_metallicroughness.texture
map_ARM -imfchan green -clamp on textures/packed_arm.texture
"
    .to_vec();
    let expected_materials = [
        b"# mtllib packed_aliases.mtl
name=ShortPacked
texture.metallic_roughness=models/textures/packed_mr.texture
texture.metallic_roughness.source_channel=green
texture.metallic_roughness.color_space=non_color
texture.mra=models/textures/packed_mra.texture
texture.mra.source_channel=red
"
        .to_vec(),
        b"# mtllib packed_aliases.mtl
name=LongPacked
texture.metallic_roughness=models/textures/packed_metallicroughness.texture
texture.metallic_roughness.source_channel=blue
texture.arm=models/textures/packed_arm.texture
texture.arm.sampler.address=clamp_to_edge
texture.arm.source_channel=green
"
        .to_vec(),
    ];
    let texture_sources = [
        texture_bytes(1, 1, 51),
        texture_bytes(1, 1, 52),
        texture_bytes(1, 1, 53),
        texture_bytes(1, 1, 54),
    ];
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/packed_aliases.mtl", material_source);
    for (path, source) in texture_paths.iter().zip(texture_sources.iter()) {
        io.insert(path.path(), source.clone());
    }
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let texture_ids = texture_paths
        .iter()
        .map(|path| database.import_asset_path(path).unwrap())
        .collect::<Vec<_>>();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_ids = mesh_paths
        .iter()
        .map(|path| database.registry().metadata_by_path(path).unwrap().id)
        .collect::<Vec<_>>();
    let material_ids = material_paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let metadata = database.registry().metadata_by_path(path).unwrap();
            assert_eq!(metadata.dependencies, texture_ids[index * 2..index * 2 + 2]);
            assert_eq!(
                fs::read(config.imported_root.join(path.path())).unwrap(),
                expected_materials[index]
            );
            metadata.id
        })
        .collect::<Vec<_>>();

    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert_eq!(
        model_dependencies.len(),
        texture_ids.len() + mesh_ids.len() + material_ids.len()
    );
    for id in texture_ids
        .iter()
        .chain(mesh_ids.iter())
        .chain(material_ids.iter())
    {
        assert!(model_dependencies.contains(id));
    }

    for id in texture_ids
        .iter()
        .chain(mesh_ids.iter())
        .chain(material_ids.iter())
    {
        database.cook_asset(*id, TargetPlatform::Windows).unwrap();
    }
    let mut bundle_ids = Vec::new();
    bundle_ids.extend(mesh_ids.iter().copied());
    bundle_ids.extend(material_ids.iter().copied());
    bundle_ids.extend(texture_ids.iter().copied());
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "pbr_packed_texture_aliases",
            bundle_ids,
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    for index in 0..material_ids.len() {
        let texture_start = index * 2;
        assert_eq!(
            reader.manifest().dependencies(mesh_ids[index]),
            Some([material_ids[index]].as_slice())
        );
        assert_eq!(
            reader.manifest().dependencies(material_ids[index]),
            Some(texture_ids[texture_start..texture_start + 2].as_ref())
        );
        assert_eq!(
            reader.read_path(&material_paths[index]).unwrap(),
            expected_materials[index]
        );
    }
    for (index, texture_path) in texture_paths.iter().enumerate() {
        assert_eq!(
            reader.manifest().dependencies(texture_ids[index]),
            Some([].as_slice())
        );
        assert_eq!(
            reader.read_path(texture_path).unwrap(),
            texture_sources[index]
        );
    }

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..12 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let expected_names = [["metallic_roughness", "mra"], ["metallic_roughness", "arm"]];
    let expected_channels = [
        [MaterialTextureChannel::Green, MaterialTextureChannel::Red],
        [MaterialTextureChannel::Blue, MaterialTextureChannel::Green],
    ];
    for index in 0..material_ids.len() {
        let texture_start = index * 2;
        assert_eq!(
            server
                .dependency_graph()
                .direct_dependencies(material_ids[index]),
            &texture_ids[texture_start..texture_start + 2]
        );
        let material = server.get_by_id::<Material>(material_ids[index]).unwrap();
        assert_eq!(material.textures.len(), 2);
        for texture_index in 0..2 {
            assert_eq!(
                material.textures[texture_index].name,
                expected_names[index][texture_index]
            );
            assert_eq!(
                material.textures[texture_index].texture.id(),
                texture_ids[texture_start + texture_index]
            );
            assert_eq!(
                material.textures[texture_index].options.source_channel,
                Some(expected_channels[index][texture_index])
            );
        }
    }
    let short = server.get_by_id::<Material>(material_ids[0]).unwrap();
    assert_eq!(
        short.textures[0].options.color_space,
        Some(MaterialTextureColorSpace::NonColor)
    );
    let long = server.get_by_id::<Material>(material_ids[1]).unwrap();
    assert_eq!(long.textures[1].sampler.address, AddressMode::ClampToEdge);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_maps_obj_packed_pbr_long_name_texture_aliases() {
    for (directive, stem, channel, texel) in [
        (
            "map_occlusionroughnessmetallic",
            "occlusionroughnessmetallic_alias",
            "orm",
            122,
        ),
        (
            "map_occlusion_roughness_metallic",
            "occlusion_roughness_metallic_alias",
            "orm",
            123,
        ),
        (
            "map_metallic_roughness",
            "metallic_roughness_alias",
            "metallic_roughness",
            124,
        ),
        (
            "map_roughnessmetallic",
            "roughnessmetallic_alias",
            "metallic_roughness",
            125,
        ),
        (
            "map_roughness_metallic",
            "roughness_metallic_alias",
            "metallic_roughness",
            126,
        ),
    ] {
        let config = database_config(&format!("builtin_model_obj_{stem}"));
        let model_path = AssetPath::parse(&format!("models/{stem}.obj"));
        let mesh_path = AssetPath::parse(&format!("models/{stem}.Panel.mesh"));
        let material_path = AssetPath::parse(&format!("models/{stem}.Material_Surface.material"));
        let texture_path = AssetPath::parse(&format!("models/textures/{stem}.texture"));
        let material_library = format!("{stem}.mtl");
        let model_source = format!(
            "mtllib {material_library}\n\
o Panel\n\
v 0 0 0\n\
v 1 0 0\n\
v 0 1 0\n\
usemtl Surface\n\
f 1 2 3\n"
        )
        .into_bytes();
        let material_source =
            format!("newmtl Surface\n{directive} textures/{stem}.texture\n").into_bytes();
        let expected_material = format!(
            "# mtllib {material_library}\n\
name=Surface\n\
texture.{channel}=models/textures/{stem}.texture\n"
        )
        .into_bytes();
        let texture_source = texture_bytes(1, 1, texel);
        let mut io = MemoryAssetIo::new();
        io.insert(model_path.path(), model_source);
        io.insert(format!("models/{material_library}"), material_source);
        io.insert(texture_path.path(), texture_source.clone());
        let mut database = AssetDatabase::new(config.clone());
        database.set_io(io);
        database.register_builtin_importers();
        database.register_builtin_cookers();

        let texture_id = database.import_asset_path(&texture_path).unwrap();
        let model_id = database.import_asset_path(&model_path).unwrap();
        let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
        let material_metadata = database
            .registry()
            .metadata_by_path(&material_path)
            .unwrap();
        let material_id = material_metadata.id;

        assert_eq!(material_metadata.labels, vec!["Material/Surface"]);
        assert_eq!(material_metadata.dependencies, vec![texture_id]);
        assert_eq!(
            fs::read(config.imported_root.join(material_path.path())).unwrap(),
            expected_material
        );
        assert_eq!(
            database.registry().get(model_id).unwrap().dependencies,
            vec![texture_id, mesh_id, material_id]
        );

        database
            .cook_asset(texture_id, TargetPlatform::Windows)
            .unwrap();
        database
            .cook_asset(mesh_id, TargetPlatform::Windows)
            .unwrap();
        database
            .cook_asset(material_id, TargetPlatform::Windows)
            .unwrap();
        let bundle = database
            .build_bundle(&AssetDatabaseBundleBuild::new(
                format!("{stem}_model"),
                vec![mesh_id, material_id, texture_id],
            ))
            .unwrap();
        let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
        assert_eq!(
            reader.manifest().dependencies(mesh_id),
            Some([material_id].as_slice())
        );
        assert_eq!(
            reader.manifest().dependencies(material_id),
            Some([texture_id].as_slice())
        );
        assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
        assert_eq!(reader.read_path(&texture_path).unwrap(), texture_source);

        let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
        let mut server = AssetServer::new(AssetServerConfig::default());
        server.set_io(bundle_io);
        server.register_builtin_loaders();
        let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
        let group = server.preload_bundle(&mounted);
        for _ in 0..8 {
            server.update_loading();
            finish_uploads(&mut server);
            if server.group_state(&group) == AssetLoadState::Ready {
                break;
            }
        }

        assert_eq!(server.group_state(&group), AssetLoadState::Ready);
        assert_eq!(
            server.dependency_graph().direct_dependencies(material_id),
            vec![texture_id]
        );
        let material = server.get_by_id::<Material>(material_id).unwrap();
        assert_eq!(material.textures.len(), 1);
        assert_eq!(material.textures[0].name, channel);
        assert_eq!(material.textures[0].texture.id(), texture_id);
    }
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_parses_obj_quoted_material_texture_paths() {
    let config = database_config("builtin_model_obj_quoted_material_texture_paths");
    let model_path = AssetPath::parse("models/quoted_texture_paths.obj");
    let mesh_path = AssetPath::parse("models/quoted_texture_paths.Panel.mesh");
    let material_path = AssetPath::parse("models/quoted_texture_paths.Material_Quoted.material");
    let albedo_path = AssetPath::parse("models/textures/painted albedo.texture");
    let specular_path = AssetPath::parse("models/textures/polished specular.texture");
    let model_source = b"mtllib quoted_texture_paths.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Quoted
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Quoted
map_Kd -clamp on \"textures/painted albedo.texture\" -imfchan red
map_Ks 'textures/polished specular.texture' -imfchan blue -colorspace Non-Color
"
    .to_vec();
    let expected_material = b"# mtllib quoted_texture_paths.mtl
name=Quoted
texture.albedo=models/textures/painted albedo.texture
texture.albedo.sampler.address=clamp_to_edge
texture.albedo.source_channel=red
texture.specular=models/textures/polished specular.texture
texture.specular.source_channel=blue
texture.specular.color_space=non_color
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 127);
    let specular_source = texture_bytes(1, 1, 128);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/quoted_texture_paths.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    io.insert(specular_path.path(), specular_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let specular_id = database.import_asset_path(&specular_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Quoted"]);
    assert_eq!(material_metadata.dependencies, vec![albedo_id, specular_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![albedo_id, specular_id, mesh_id, material_id]
    );

    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(specular_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "quoted_texture_paths",
            vec![mesh_id, material_id, albedo_id, specular_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id, specular_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);
    assert_eq!(reader.read_path(&specular_path).unwrap(), specular_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        vec![albedo_id, specular_id]
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 2);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(
        material.textures[0].sampler.address,
        AddressMode::ClampToEdge
    );
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Red)
    );
    assert_eq!(material.textures[1].name, "specular");
    assert_eq!(material.textures[1].texture.id(), specular_id);
    assert_eq!(
        material.textures[1].options.source_channel,
        Some(MaterialTextureChannel::Blue)
    );
    assert_eq!(
        material.textures[1].options.color_space,
        Some(MaterialTextureColorSpace::NonColor)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_parses_obj_quoted_material_names_and_libraries() {
    let config = database_config("builtin_model_obj_quoted_material_names");
    let model_path = AssetPath::parse("models/quoted_material_names.obj");
    let mesh_path = AssetPath::parse("models/quoted_material_names.Display_Panel.mesh");
    let material_path =
        AssetPath::parse("models/quoted_material_names.Material_Brushed_Metal.material");
    let model_source = b"mtllib \"material libraries/brushed palette.mtl\"
o \"Display Panel\"
v 0 0 0
v 1 0 0
v 0 1 0
usemtl 'Brushed Metal'
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl 'Brushed Metal'
Kd 0.25 0.5 0.75
"
    .to_vec();
    let expected_material = b"# mtllib material libraries/brushed palette.mtl
name=Brushed Metal
base_color=0.25,0.5,0.75,1
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert(
        "models/material libraries/brushed palette.mtl",
        material_source,
    );
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(mesh_metadata.labels, vec!["Display Panel"]);
    assert_eq!(mesh_metadata.dependencies, vec![material_id]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(material_metadata.labels, vec!["Material/Brushed Metal"]);
    assert!(material_metadata.dependencies.is_empty());
    assert_eq!(material_metadata.importer_version, 111);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, material_id]
    );

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "quoted_material_names",
            vec![mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(mesh_id),
        vec![material_id]
    );
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        Vec::<AssetId>::new()
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.properties.base_color, [0.25, 0.5, 0.75, 1.0]);
    assert!(server.get_by_id::<Mesh>(mesh_id).is_some());
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_hash_inside_quoted_obj_names() {
    let config = database_config("builtin_model_obj_quoted_hash_names");
    let model_path = AssetPath::parse("models/hash_comments.obj");
    let mesh_path = AssetPath::parse("models/hash_comments.Panel__A.mesh");
    let material_path = AssetPath::parse("models/hash_comments.Material_Hash___Metal.material");
    let model_source = b"mtllib hash_comments.mtl # material library comment
o \"Panel #A\" # object label comment
v 0 0 0
v 1 0 0
v 0 1 0
usemtl \"Hash # Metal\" # material binding comment
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl \"Hash # Metal\" # material name comment
Kd 0.6 0.4 0.2 # base color comment
"
    .to_vec();
    let expected_material = b"# mtllib hash_comments.mtl
name=Hash # Metal
base_color=0.6,0.4,0.2,1
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/hash_comments.mtl", material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(mesh_metadata.labels, vec!["Panel #A"]);
    assert_eq!(mesh_metadata.dependencies, vec![material_id]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(material_metadata.labels, vec!["Material/Hash # Metal"]);
    assert!(material_metadata.dependencies.is_empty());
    assert_eq!(material_metadata.importer_version, 111);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, material_id]
    );

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "quoted_hash_names",
            vec![mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(mesh_id),
        vec![material_id]
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.properties.base_color, [0.6, 0.4, 0.2, 1.0]);
    assert!(server.get_by_id::<Mesh>(mesh_id).is_some());
}

#[test]
fn database_model_importer_reports_unterminated_obj_quote() {
    let config = database_config("builtin_model_obj_unterminated_quote");
    let model_path = AssetPath::parse("models/bad_obj_quote.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib \"bad material.mtl
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_obj_quote.obj")
                && message.contains("OBJ source has unterminated \" quote on line 1")
    ));
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_parses_obj_legacy_texture_map_extensions() {
    let config = database_config("builtin_model_obj_legacy_texture_maps");
    let model_path = AssetPath::parse("models/legacy_maps.obj");
    let mesh_path = AssetPath::parse("models/legacy_maps.Panel.mesh");
    let material_path = AssetPath::parse("models/legacy_maps.Material_Legacy.material");
    let displacement_path = AssetPath::parse("models/textures/height.texture");
    let displacement_alias_path = AssetPath::parse("models/textures/height_alias.texture");
    let decal_path = AssetPath::parse("models/textures/decal.texture");
    let decal_alias_path = AssetPath::parse("models/textures/decal_alias.texture");
    let reflection_path = AssetPath::parse("models/textures/reflection.texture");
    let reflection_alias_path = AssetPath::parse("models/textures/reflection_alias.texture");
    let model_source = b"mtllib legacy_maps.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Legacy
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Legacy
DISP textures/height.texture
MAP_DISP textures/height_alias.texture
DECAL textures/decal.texture
MAP_DECAL textures/decal_alias.texture
refl -type CUBE_TOP textures/reflection.texture
MAP_REFL -type Cube_Top textures/reflection_alias.texture
"
    .to_vec();
    let expected_material = b"# mtllib legacy_maps.mtl
name=Legacy
texture.displacement=models/textures/height_alias.texture
texture.decal=models/textures/decal_alias.texture
texture.reflection=models/textures/reflection_alias.texture
texture.reflection.projection=cube_top
"
    .to_vec();
    let displacement_source = texture_bytes(1, 1, 21);
    let displacement_alias_source = texture_bytes(1, 1, 24);
    let decal_source = texture_bytes(1, 1, 22);
    let decal_alias_source = texture_bytes(1, 1, 25);
    let reflection_source = texture_bytes(1, 1, 23);
    let reflection_alias_source = texture_bytes(1, 1, 26);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/legacy_maps.mtl", material_source);
    io.insert(displacement_path.path(), displacement_source.clone());
    io.insert(
        displacement_alias_path.path(),
        displacement_alias_source.clone(),
    );
    io.insert(decal_path.path(), decal_source.clone());
    io.insert(decal_alias_path.path(), decal_alias_source.clone());
    io.insert(reflection_path.path(), reflection_source.clone());
    io.insert(
        reflection_alias_path.path(),
        reflection_alias_source.clone(),
    );
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let displacement_id = database.import_asset_path(&displacement_path).unwrap();
    let displacement_alias_id = database
        .import_asset_path(&displacement_alias_path)
        .unwrap();
    let decal_id = database.import_asset_path(&decal_path).unwrap();
    let decal_alias_id = database.import_asset_path(&decal_alias_path).unwrap();
    let reflection_id = database.import_asset_path(&reflection_path).unwrap();
    let reflection_alias_id = database.import_asset_path(&reflection_alias_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Legacy"]);
    assert_eq!(
        material_metadata.dependencies,
        vec![displacement_alias_id, decal_alias_id, reflection_alias_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![
            displacement_alias_id,
            decal_alias_id,
            reflection_alias_id,
            mesh_id,
            material_id
        ]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![displacement_alias_id, decal_alias_id, reflection_alias_id]
    );

    for texture_id in [
        displacement_id,
        displacement_alias_id,
        decal_id,
        decal_alias_id,
        reflection_id,
        reflection_alias_id,
    ] {
        database
            .cook_asset(texture_id, TargetPlatform::Windows)
            .unwrap();
    }
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "legacy_texture_maps",
            vec![
                mesh_id,
                material_id,
                displacement_id,
                displacement_alias_id,
                decal_id,
                decal_alias_id,
                reflection_id,
                reflection_alias_id,
            ],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([displacement_alias_id, decal_alias_id, reflection_alias_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(
        reader.read_path(&displacement_path).unwrap(),
        displacement_source
    );
    assert_eq!(
        reader.read_path(&displacement_alias_path).unwrap(),
        displacement_alias_source
    );
    assert_eq!(reader.read_path(&decal_path).unwrap(), decal_source);
    assert_eq!(
        reader.read_path(&decal_alias_path).unwrap(),
        decal_alias_source
    );
    assert_eq!(
        reader.read_path(&reflection_path).unwrap(),
        reflection_source
    );
    assert_eq!(
        reader.read_path(&reflection_alias_path).unwrap(),
        reflection_alias_source
    );

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(material_id),
        vec![displacement_alias_id, decal_alias_id, reflection_alias_id]
    );
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 3);
    assert_eq!(material.textures[0].name, "displacement");
    assert_eq!(material.textures[0].texture.id(), displacement_alias_id);
    assert_eq!(material.textures[1].name, "decal");
    assert_eq!(material.textures[1].texture.id(), decal_alias_id);
    assert_eq!(material.textures[2].name, "reflection");
    assert_eq!(material.textures[2].texture.id(), reflection_alias_id);
    assert_eq!(
        material.textures[2].options.projection,
        Some(MaterialTextureProjection::CubeTop)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_reflection_sphere_projection() {
    let config = database_config("builtin_model_obj_reflection_sphere_projection");
    let model_path = AssetPath::parse("models/reflection_sphere.obj");
    let mesh_path = AssetPath::parse("models/reflection_sphere.Panel.mesh");
    let material_path = AssetPath::parse("models/reflection_sphere.Material_Reflect.material");
    let reflection_path = AssetPath::parse("models/textures/reflection.texture");
    let model_source = b"mtllib reflection_sphere.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Reflect
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Reflect
refl -type sphere textures/reflection.texture
"
    .to_vec();
    let expected_material = b"# mtllib reflection_sphere.mtl
name=Reflect
texture.reflection=models/textures/reflection.texture
texture.reflection.projection=sphere
"
    .to_vec();
    let reflection_source = texture_bytes(1, 1, 27);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/reflection_sphere.mtl", material_source);
    io.insert(reflection_path.path(), reflection_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let reflection_id = database.import_asset_path(&reflection_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Reflect"]);
    assert_eq!(material_metadata.dependencies, vec![reflection_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![reflection_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![reflection_id]
    );

    database
        .cook_asset(reflection_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "reflection_sphere_projection",
            vec![mesh_id, material_id, reflection_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([reflection_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(
        reader.read_path(&reflection_path).unwrap(),
        reflection_source
    );

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "reflection");
    assert_eq!(material.textures[0].texture.id(), reflection_id);
    assert_eq!(
        material.textures[0].options.projection,
        Some(MaterialTextureProjection::Sphere)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_material_custom_properties() {
    let config = database_config("builtin_model_obj_material_custom_properties");
    let model_path = AssetPath::parse("models/material_props.obj");
    let mesh_path = AssetPath::parse("models/material_props.Panel.mesh");
    let material_path = AssetPath::parse("models/material_props.Material_Rich.material");
    let model_source = b"mtllib material_props.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Rich
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Rich
kA 0.05 0.06 0.07
KS 0.8 0.7 0.6
tF 0.9 0.95 1.0
NI 1.45
ILLUM 7
kd 0.2 0.3 0.4
"
    .to_vec();
    let expected_material = b"# mtllib material_props.mtl
name=Rich
custom.ambient_color.vec3=0.05,0.06,0.07
custom.specular_color.vec3=0.8,0.7,0.6
custom.transmission_filter.vec3=0.9,0.95,1
custom.index_of_refraction.float=1.45
custom.illumination_model.int=7
base_color=0.2,0.3,0.4,1
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/material_props.mtl", material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Rich"]);
    assert!(material_metadata.dependencies.is_empty());
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        Vec::<AssetId>::new()
    );

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_custom_properties",
            vec![mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.properties.base_color, [0.2, 0.3, 0.4, 1.0]);
    assert_eq!(
        material.properties.custom.get("ambient_color"),
        Some(&MaterialPropertyValue::Vec3([0.05, 0.06, 0.07]))
    );
    assert_eq!(
        material.properties.custom.get("specular_color"),
        Some(&MaterialPropertyValue::Vec3([0.8, 0.7, 0.6]))
    );
    assert_eq!(
        material.properties.custom.get("transmission_filter"),
        Some(&MaterialPropertyValue::Vec3([0.9, 0.95, 1.0]))
    );
    assert_eq!(
        material.properties.custom.get("index_of_refraction"),
        Some(&MaterialPropertyValue::Float(1.45))
    );
    assert_eq!(
        material.properties.custom.get("illumination_model"),
        Some(&MaterialPropertyValue::Int(7))
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_converts_obj_xyz_material_colors() {
    let config = database_config("builtin_model_obj_xyz_material_colors");
    let model_path = AssetPath::parse("models/xyz_colors.obj");
    let mesh_path = AssetPath::parse("models/xyz_colors.Panel.mesh");
    let material_path = AssetPath::parse("models/xyz_colors.Material_Xyz.material");
    let model_source = b"mtllib xyz_colors.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Xyz
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Xyz
Kd xyz 0.95047 1.0 1.08883
Ka XYZ 0.190094 0.2 0.217766
Ks xyz 0.285141 0.3 0.326649
Ke xyz 0.095047 0.1 0.108883
Tf xyz 0.475235 0.5 0.544415
"
    .to_vec();
    let expected_material = b"# mtllib xyz_colors.mtl
name=Xyz
custom.ambient_color.vec3=0.20000045,0.20001522,0.19996691
custom.specular_color.vec3=0.30000073,0.30002287,0.29995036
custom.transmission_filter.vec3=0.50000125,0.500038,0.49991727
base_color=1.0000025,1.000076,0.99983454,1
emissive=0.100000225,0.10000761,0.099983454
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/xyz_colors.mtl", material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Xyz"]);
    assert!(material_metadata.dependencies.is_empty());
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "xyz_material_colors",
            vec![mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(
        material.properties.base_color,
        [1.0000025, 1.000076, 0.99983454, 1.0]
    );
    assert_eq!(
        material.properties.emissive,
        [0.100000225, 0.10000761, 0.099983454]
    );
    assert_eq!(
        material.properties.custom.get("ambient_color"),
        Some(&MaterialPropertyValue::Vec3([
            0.20000045, 0.20001522, 0.19996691
        ]))
    );
    assert_eq!(
        material.properties.custom.get("specular_color"),
        Some(&MaterialPropertyValue::Vec3([
            0.30000073, 0.30002287, 0.29995036
        ]))
    );
    assert_eq!(
        material.properties.custom.get("transmission_filter"),
        Some(&MaterialPropertyValue::Vec3([
            0.50000125, 0.500038, 0.49991727
        ]))
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_parses_obj_ior_alias_custom_property() {
    let config = database_config("builtin_model_obj_material_ior_alias");
    let model_path = AssetPath::parse("models/material_ior_alias.obj");
    let mesh_path = AssetPath::parse("models/material_ior_alias.Panel.mesh");
    let material_path = AssetPath::parse("models/material_ior_alias.Material_Ior.material");
    let model_source = b"mtllib material_ior_alias.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Ior
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Ior
ior 1.33
"
    .to_vec();
    let expected_material = b"# mtllib material_ior_alias.mtl
name=Ior
custom.index_of_refraction.float=1.33
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/material_ior_alias.mtl", material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Ior"]);
    assert!(material_metadata.dependencies.is_empty());
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        Vec::<AssetId>::new()
    );

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_ior_alias",
            vec![mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(
        material.properties.custom.get("index_of_refraction"),
        Some(&MaterialPropertyValue::Float(1.33))
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_ambient_and_specular_texture_maps() {
    let config = database_config("builtin_model_obj_ambient_specular_texture_maps");
    let model_path = AssetPath::parse("models/material_maps.obj");
    let mesh_path = AssetPath::parse("models/material_maps.Panel.mesh");
    let material_path = AssetPath::parse("models/material_maps.Material_Lit.material");
    let occlusion_path = AssetPath::parse("models/textures/material_occlusion.texture");
    let specular_path = AssetPath::parse("models/textures/material_specular.texture");
    let model_source = b"mtllib material_maps.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Lit
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Lit
map_Ka textures/material_occlusion.texture
map_Ks -imfchan BLUE -colorspace Non-Color textures/material_specular.texture
"
    .to_vec();
    let expected_material = b"# mtllib material_maps.mtl
name=Lit
texture.occlusion=models/textures/material_occlusion.texture
texture.specular=models/textures/material_specular.texture
texture.specular.source_channel=blue
texture.specular.color_space=non_color
"
    .to_vec();
    let occlusion_source = texture_bytes(1, 1, 101);
    let specular_source = texture_bytes(1, 1, 102);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/material_maps.mtl", material_source);
    io.insert(occlusion_path.path(), occlusion_source.clone());
    io.insert(specular_path.path(), specular_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let occlusion_id = database.import_asset_path(&occlusion_path).unwrap();
    let specular_id = database.import_asset_path(&specular_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Lit"]);
    assert!(material_metadata.dependencies.contains(&occlusion_id));
    assert!(material_metadata.dependencies.contains(&specular_id));
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));
    let material_dependencies = &database.registry().get(material_id).unwrap().dependencies;
    assert!(material_dependencies.contains(&occlusion_id));
    assert!(material_dependencies.contains(&specular_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    let loaded_material_dependencies = &loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies;
    assert!(loaded_material_dependencies.contains(&occlusion_id));
    assert!(loaded_material_dependencies.contains(&specular_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(occlusion_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(specular_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_ambient_specular_maps",
            vec![mesh_id, material_id, occlusion_id, specular_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&occlusion_id));
    assert!(bundle_material_dependencies.contains(&specular_id));
    assert_eq!(
        reader.manifest().dependencies(occlusion_id),
        Some([].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(specular_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "occlusion");
    assert_eq!(material.textures[0].texture.id(), occlusion_id);
    assert_eq!(material.textures[1].name, "specular");
    assert_eq!(material.textures[1].texture.id(), specular_id);
    assert_eq!(
        material.textures[1].options.source_channel,
        Some(MaterialTextureChannel::Blue)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_uppercase_and_plain_bump_aliases() {
    let config = database_config("builtin_model_obj_uppercase_bump_alias");
    let model_path = AssetPath::parse("models/bump_alias.obj");
    let mesh_path = AssetPath::parse("models/bump_alias.Panel.mesh");
    let material_path = AssetPath::parse("models/bump_alias.Material_Detail.material");
    let upper_bump_path = AssetPath::parse("models/textures/detail_upper.texture");
    let model_source = b"mtllib bump_alias.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Detail
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Detail
map_Bump textures/detail_upper.texture
"
    .to_vec();
    let expected_material = b"# mtllib bump_alias.mtl
name=Detail
texture.normal=models/textures/detail_upper.texture
"
    .to_vec();
    let upper_bump_source = texture_bytes(1, 1, 103);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/bump_alias.mtl", material_source);
    io.insert(upper_bump_path.path(), upper_bump_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let upper_bump_id = database.import_asset_path(&upper_bump_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Detail"]);
    assert!(material_metadata.dependencies.contains(&upper_bump_id));
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));
    let material_dependencies = &database.registry().get(material_id).unwrap().dependencies;
    assert!(material_dependencies.contains(&upper_bump_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    let loaded_material_dependencies = &loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies;
    assert!(loaded_material_dependencies.contains(&upper_bump_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(upper_bump_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_uppercase_bump_aliases",
            vec![mesh_id, material_id, upper_bump_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&upper_bump_id));
    assert_eq!(
        reader.manifest().dependencies(upper_bump_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "normal");
    assert_eq!(material.textures[0].texture.id(), upper_bump_id);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_plain_bump_alias() {
    let config = database_config("builtin_model_obj_plain_bump_alias");
    let model_path = AssetPath::parse("models/plain_bump.obj");
    let mesh_path = AssetPath::parse("models/plain_bump.Panel.mesh");
    let material_path = AssetPath::parse("models/plain_bump.Material_Detail.material");
    let bump_path = AssetPath::parse("models/textures/detail.texture");
    let model_source = b"mtllib plain_bump.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Detail
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Detail
bump textures/detail.texture
"
    .to_vec();
    let expected_material = b"# mtllib plain_bump.mtl
name=Detail
texture.normal=models/textures/detail.texture
"
    .to_vec();
    let bump_source = texture_bytes(1, 1, 105);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/plain_bump.mtl", material_source);
    io.insert(bump_path.path(), bump_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let bump_id = database.import_asset_path(&bump_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Detail"]);
    assert!(material_metadata.dependencies.contains(&bump_id));
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));
    let material_dependencies = &database.registry().get(material_id).unwrap().dependencies;
    assert!(material_dependencies.contains(&bump_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    let loaded_material_dependencies = &loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies;
    assert!(loaded_material_dependencies.contains(&bump_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(bump_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_plain_bump_alias",
            vec![mesh_id, material_id, bump_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&bump_id));
    assert_eq!(reader.manifest().dependencies(bump_id), Some([].as_slice()));
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "normal");
    assert_eq!(material.textures[0].texture.id(), bump_id);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_norm_alias() {
    let config = database_config("builtin_model_obj_norm_alias");
    let model_path = AssetPath::parse("models/norm_alias.obj");
    let mesh_path = AssetPath::parse("models/norm_alias.Panel.mesh");
    let material_path = AssetPath::parse("models/norm_alias.Material_Detail.material");
    let norm_path = AssetPath::parse("models/textures/detail_norm.texture");
    let model_source = b"mtllib norm_alias.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Detail
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Detail
norm textures/detail_norm.texture
"
    .to_vec();
    let expected_material = b"# mtllib norm_alias.mtl
name=Detail
texture.normal=models/textures/detail_norm.texture
"
    .to_vec();
    let norm_source = texture_bytes(1, 1, 107);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/norm_alias.mtl", material_source);
    io.insert(norm_path.path(), norm_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let norm_id = database.import_asset_path(&norm_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Detail"]);
    assert!(material_metadata.dependencies.contains(&norm_id));
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));
    let material_dependencies = &database.registry().get(material_id).unwrap().dependencies;
    assert!(material_dependencies.contains(&norm_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    let loaded_material_dependencies = &loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies;
    assert!(loaded_material_dependencies.contains(&norm_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(norm_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_norm_alias",
            vec![mesh_id, material_id, norm_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&norm_id));
    assert_eq!(reader.manifest().dependencies(norm_id), Some([].as_slice()));
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "normal");
    assert_eq!(material.textures[0].texture.id(), norm_id);
}

#[test]
fn database_model_importer_preserves_obj_normal_map_aliases() {
    let config = database_config("builtin_model_obj_normal_map_aliases");
    let model_path = AssetPath::parse("models/normal_map_aliases.obj");
    let kn_material_path = AssetPath::parse("models/normal_map_aliases.Material_Kn.material");
    let normal_material_path =
        AssetPath::parse("models/normal_map_aliases.Material_Normal.material");
    let kn_texture_path = AssetPath::parse("models/textures/kn_normal.texture");
    let normal_texture_path = AssetPath::parse("models/textures/normal_map.texture");
    let model_source = b"mtllib normal_map_aliases.mtl
o KnPanel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Kn
f 1 2 3
o NormalPanel
usemtl Normal
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Kn
map_Kn -bm 0.4 textures/kn_normal.texture
newmtl Normal
map_normal textures/normal_map.texture
"
    .to_vec();
    let expected_kn_material = b"# mtllib normal_map_aliases.mtl
name=Kn
texture.normal=models/textures/kn_normal.texture
texture.normal.bump_scale=0.4
"
    .to_vec();
    let expected_normal_material = b"# mtllib normal_map_aliases.mtl
name=Normal
texture.normal=models/textures/normal_map.texture
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/normal_map_aliases.mtl", material_source);
    io.insert(kn_texture_path.path(), texture_bytes(1, 1, 109));
    io.insert(normal_texture_path.path(), texture_bytes(1, 1, 110));
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let kn_texture_id = database.import_asset_path(&kn_texture_path).unwrap();
    let normal_texture_id = database.import_asset_path(&normal_texture_path).unwrap();
    let _model_id = database.import_asset_path(&model_path).unwrap();
    let kn_material_metadata = database
        .registry()
        .metadata_by_path(&kn_material_path)
        .unwrap();
    let normal_material_metadata = database
        .registry()
        .metadata_by_path(&normal_material_path)
        .unwrap();

    assert_eq!(kn_material_metadata.dependencies, vec![kn_texture_id]);
    assert_eq!(
        normal_material_metadata.dependencies,
        vec![normal_texture_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(kn_material_path.path())).unwrap(),
        expected_kn_material
    );
    assert_eq!(
        fs::read(config.imported_root.join(normal_material_path.path())).unwrap(),
        expected_normal_material
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_emissive_texture_map_alias() {
    let config = database_config("builtin_model_obj_emissive_texture_map_alias");
    let model_path = AssetPath::parse("models/emissive_map.obj");
    let mesh_path = AssetPath::parse("models/emissive_map.Panel.mesh");
    let material_path = AssetPath::parse("models/emissive_map.Material_Glow.material");
    let emissive_path = AssetPath::parse("models/textures/glow.texture");
    let model_source = b"mtllib emissive_map.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Glow
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Glow
map_Ke -type sphere -imfchan r textures/glow.texture
"
    .to_vec();
    let expected_material = b"# mtllib emissive_map.mtl
name=Glow
texture.emissive=models/textures/glow.texture
texture.emissive.source_channel=red
texture.emissive.projection=sphere
"
    .to_vec();
    let emissive_source = texture_bytes(1, 1, 106);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/emissive_map.mtl", material_source);
    io.insert(emissive_path.path(), emissive_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let emissive_id = database.import_asset_path(&emissive_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Glow"]);
    assert!(material_metadata.dependencies.contains(&emissive_id));
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));
    let material_dependencies = &database.registry().get(material_id).unwrap().dependencies;
    assert!(material_dependencies.contains(&emissive_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    let loaded_material_dependencies = &loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies;
    assert!(loaded_material_dependencies.contains(&emissive_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(emissive_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_emissive_map_alias",
            vec![mesh_id, material_id, emissive_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&emissive_id));
    assert_eq!(
        reader.manifest().dependencies(emissive_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "emissive");
    assert_eq!(material.textures[0].texture.id(), emissive_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Red)
    );
    assert_eq!(
        material.textures[0].options.projection,
        Some(MaterialTextureProjection::Sphere)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_metallic_and_roughness_texture_maps() {
    let config = database_config("builtin_model_obj_metallic_roughness_texture_maps");
    let model_path = AssetPath::parse("models/metallic_roughness.obj");
    let mesh_path = AssetPath::parse("models/metallic_roughness.Panel.mesh");
    let material_path = AssetPath::parse("models/metallic_roughness.Material_Metal.material");
    let roughness_path = AssetPath::parse("models/textures/roughness.texture");
    let metallic_path = AssetPath::parse("models/textures/metallic.texture");
    let model_source = b"mtllib metallic_roughness.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Metal
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Metal
map_Pr -imfchan green textures/roughness.texture
map_Pm -imfchan red textures/metallic.texture
"
    .to_vec();
    let expected_material = b"# mtllib metallic_roughness.mtl
name=Metal
texture.roughness=models/textures/roughness.texture
texture.roughness.source_channel=green
texture.metallic=models/textures/metallic.texture
texture.metallic.source_channel=red
"
    .to_vec();
    let roughness_source = texture_bytes(1, 1, 108);
    let metallic_source = texture_bytes(1, 1, 109);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/metallic_roughness.mtl", material_source);
    io.insert(roughness_path.path(), roughness_source.clone());
    io.insert(metallic_path.path(), metallic_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let roughness_id = database.import_asset_path(&roughness_path).unwrap();
    let metallic_id = database.import_asset_path(&metallic_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Metal"]);
    assert!(material_metadata.dependencies.contains(&roughness_id));
    assert!(material_metadata.dependencies.contains(&metallic_id));
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));
    let material_dependencies = &database.registry().get(material_id).unwrap().dependencies;
    assert!(material_dependencies.contains(&roughness_id));
    assert!(material_dependencies.contains(&metallic_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    let loaded_material_dependencies = &loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies;
    assert!(loaded_material_dependencies.contains(&roughness_id));
    assert!(loaded_material_dependencies.contains(&metallic_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(roughness_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(metallic_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_metallic_roughness_maps",
            vec![mesh_id, material_id, roughness_id, metallic_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&roughness_id));
    assert!(bundle_material_dependencies.contains(&metallic_id));
    assert_eq!(
        reader.manifest().dependencies(roughness_id),
        Some([].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(metallic_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "roughness");
    assert_eq!(material.textures[0].texture.id(), roughness_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Green)
    );
    assert_eq!(material.textures[1].name, "metallic");
    assert_eq!(material.textures[1].texture.id(), metallic_id);
    assert_eq!(
        material.textures[1].options.source_channel,
        Some(MaterialTextureChannel::Red)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_luminance_source_channel() {
    let config = database_config("builtin_model_obj_luminance_source_channel");
    let model_path = AssetPath::parse("models/luminance.obj");
    let mesh_path = AssetPath::parse("models/luminance.Panel.mesh");
    let material_path = AssetPath::parse("models/luminance.Material_Light.material");
    let albedo_path = AssetPath::parse("models/textures/luminance.texture");
    let model_source = b"mtllib luminance.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Light
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Light
map_Kd -imfchan Luminance textures/luminance.texture
"
    .to_vec();
    let expected_material = b"# mtllib luminance.mtl
name=Light
texture.albedo=models/textures/luminance.texture
texture.albedo.source_channel=luminance
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 110);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/luminance.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Light"]);
    assert!(material_metadata.dependencies.contains(&albedo_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&mesh_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&material_id));
    assert!(database
        .registry()
        .get(material_id)
        .unwrap()
        .dependencies
        .contains(&albedo_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert!(loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies
        .contains(&albedo_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_luminance_source_channel",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&albedo_id));
    assert_eq!(
        reader.manifest().dependencies(albedo_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Luminance)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_depth_source_channel() {
    let config = database_config("builtin_model_obj_depth_source_channel");
    let model_path = AssetPath::parse("models/depth.obj");
    let mesh_path = AssetPath::parse("models/depth.Panel.mesh");
    let material_path = AssetPath::parse("models/depth.Material_Light.material");
    let albedo_path = AssetPath::parse("models/textures/depth.texture");
    let model_source = b"mtllib depth.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Light
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Light
map_Kd -imfchan depth textures/depth.texture
"
    .to_vec();
    let expected_material = b"# mtllib depth.mtl
name=Light
texture.albedo=models/textures/depth.texture
texture.albedo.source_channel=depth
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 111);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/depth.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Light"]);
    assert!(material_metadata.dependencies.contains(&albedo_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&mesh_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&material_id));
    assert!(database
        .registry()
        .get(material_id)
        .unwrap()
        .dependencies
        .contains(&albedo_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert!(loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies
        .contains(&albedo_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_depth_source_channel",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&albedo_id));
    assert_eq!(
        reader.manifest().dependencies(albedo_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Depth)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_matte_source_channel() {
    let config = database_config("builtin_model_obj_matte_source_channel");
    let model_path = AssetPath::parse("models/matte.obj");
    let mesh_path = AssetPath::parse("models/matte.Panel.mesh");
    let material_path = AssetPath::parse("models/matte.Material_Light.material");
    let albedo_path = AssetPath::parse("models/textures/matte.texture");
    let model_source = b"mtllib matte.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Light
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Light
map_Kd -imfchan matte textures/matte.texture
"
    .to_vec();
    let expected_material = b"# mtllib matte.mtl
name=Light
texture.albedo=models/textures/matte.texture
texture.albedo.source_channel=matte
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 112);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/matte.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Light"]);
    assert!(material_metadata.dependencies.contains(&albedo_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&mesh_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&material_id));
    assert!(database
        .registry()
        .get(material_id)
        .unwrap()
        .dependencies
        .contains(&albedo_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert!(loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies
        .contains(&albedo_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_matte_source_channel",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&albedo_id));
    assert_eq!(
        reader.manifest().dependencies(albedo_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Matte)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_blue_source_channel() {
    let config = database_config("builtin_model_obj_blue_source_channel");
    let model_path = AssetPath::parse("models/blue.obj");
    let mesh_path = AssetPath::parse("models/blue.Panel.mesh");
    let material_path = AssetPath::parse("models/blue.Material_Light.material");
    let albedo_path = AssetPath::parse("models/textures/blue.texture");
    let model_source = b"mtllib blue.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Light
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Light
map_Kd -imfchan blue textures/blue.texture
"
    .to_vec();
    let expected_material = b"# mtllib blue.mtl
name=Light
texture.albedo=models/textures/blue.texture
texture.albedo.source_channel=blue
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 113);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/blue.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Light"]);
    assert!(material_metadata.dependencies.contains(&albedo_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&mesh_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&material_id));
    assert!(database
        .registry()
        .get(material_id)
        .unwrap()
        .dependencies
        .contains(&albedo_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert!(loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies
        .contains(&albedo_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_blue_source_channel",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&albedo_id));
    assert_eq!(
        reader.manifest().dependencies(albedo_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(
        material.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Blue)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_flat_projection() {
    let config = database_config("builtin_model_obj_flat_projection");
    let model_path = AssetPath::parse("models/flat.obj");
    let mesh_path = AssetPath::parse("models/flat.Panel.mesh");
    let material_path = AssetPath::parse("models/flat.Material_Reflect.material");
    let reflection_path = AssetPath::parse("models/textures/flat_reflection.texture");
    let model_source = b"mtllib flat.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Reflect
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Reflect
refl -type flat textures/flat_reflection.texture
"
    .to_vec();
    let expected_material = b"# mtllib flat.mtl
name=Reflect
texture.reflection=models/textures/flat_reflection.texture
texture.reflection.projection=flat
"
    .to_vec();
    let reflection_source = texture_bytes(1, 1, 114);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/flat.mtl", material_source);
    io.insert(reflection_path.path(), reflection_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let reflection_id = database.import_asset_path(&reflection_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Reflect"]);
    assert!(material_metadata.dependencies.contains(&reflection_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&mesh_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&material_id));
    assert!(database
        .registry()
        .get(material_id)
        .unwrap()
        .dependencies
        .contains(&reflection_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert!(loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies
        .contains(&reflection_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(reflection_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_flat_projection",
            vec![mesh_id, material_id, reflection_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&reflection_id));
    assert_eq!(
        reader.manifest().dependencies(reflection_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "reflection");
    assert_eq!(material.textures[0].texture.id(), reflection_id);
    assert_eq!(
        material.textures[0].options.projection,
        Some(MaterialTextureProjection::Flat)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_cube_bottom_projection() {
    let config = database_config("builtin_model_obj_cube_bottom_projection");
    let model_path = AssetPath::parse("models/cube_bottom.obj");
    let mesh_path = AssetPath::parse("models/cube_bottom.Panel.mesh");
    let material_path = AssetPath::parse("models/cube_bottom.Material_Reflect.material");
    let reflection_path = AssetPath::parse("models/textures/cube_bottom_reflection.texture");
    let model_source = b"mtllib cube_bottom.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Reflect
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Reflect
map_refl -type cube_bottom textures/cube_bottom_reflection.texture
"
    .to_vec();
    let expected_material = b"# mtllib cube_bottom.mtl
name=Reflect
texture.reflection=models/textures/cube_bottom_reflection.texture
texture.reflection.projection=cube_bottom
"
    .to_vec();
    let reflection_source = texture_bytes(1, 1, 115);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/cube_bottom.mtl", material_source);
    io.insert(reflection_path.path(), reflection_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let reflection_id = database.import_asset_path(&reflection_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Reflect"]);
    assert!(material_metadata.dependencies.contains(&reflection_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&mesh_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&material_id));
    assert!(database
        .registry()
        .get(material_id)
        .unwrap()
        .dependencies
        .contains(&reflection_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert!(loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies
        .contains(&reflection_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(reflection_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_cube_bottom_projection",
            vec![mesh_id, material_id, reflection_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&reflection_id));
    assert_eq!(
        reader.manifest().dependencies(reflection_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "reflection");
    assert_eq!(material.textures[0].texture.id(), reflection_id);
    assert_eq!(
        material.textures[0].options.projection,
        Some(MaterialTextureProjection::CubeBottom)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_cube_front_projection() {
    let config = database_config("builtin_model_obj_cube_front_projection");
    let model_path = AssetPath::parse("models/cube_front.obj");
    let mesh_path = AssetPath::parse("models/cube_front.Panel.mesh");
    let material_path = AssetPath::parse("models/cube_front.Material_Reflect.material");
    let reflection_path = AssetPath::parse("models/textures/cube_front_reflection.texture");
    let model_source = b"mtllib cube_front.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Reflect
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Reflect
map_refl -type cube_front textures/cube_front_reflection.texture
"
    .to_vec();
    let expected_material = b"# mtllib cube_front.mtl
name=Reflect
texture.reflection=models/textures/cube_front_reflection.texture
texture.reflection.projection=cube_front
"
    .to_vec();
    let reflection_source = texture_bytes(1, 1, 116);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/cube_front.mtl", material_source);
    io.insert(reflection_path.path(), reflection_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let reflection_id = database.import_asset_path(&reflection_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Reflect"]);
    assert!(material_metadata.dependencies.contains(&reflection_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&mesh_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&material_id));
    assert!(database
        .registry()
        .get(material_id)
        .unwrap()
        .dependencies
        .contains(&reflection_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert!(loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies
        .contains(&reflection_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(reflection_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_cube_front_projection",
            vec![mesh_id, material_id, reflection_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&reflection_id));
    assert_eq!(
        reader.manifest().dependencies(reflection_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "reflection");
    assert_eq!(material.textures[0].texture.id(), reflection_id);
    assert_eq!(
        material.textures[0].options.projection,
        Some(MaterialTextureProjection::CubeFront)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_cube_back_projection() {
    let config = database_config("builtin_model_obj_cube_back_projection");
    let model_path = AssetPath::parse("models/cube_back.obj");
    let mesh_path = AssetPath::parse("models/cube_back.Panel.mesh");
    let material_path = AssetPath::parse("models/cube_back.Material_Reflect.material");
    let reflection_path = AssetPath::parse("models/textures/cube_back_reflection.texture");
    let model_source = b"mtllib cube_back.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Reflect
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Reflect
map_refl -type cube_back textures/cube_back_reflection.texture
"
    .to_vec();
    let expected_material = b"# mtllib cube_back.mtl
name=Reflect
texture.reflection=models/textures/cube_back_reflection.texture
texture.reflection.projection=cube_back
"
    .to_vec();
    let reflection_source = texture_bytes(1, 1, 117);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/cube_back.mtl", material_source);
    io.insert(reflection_path.path(), reflection_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let reflection_id = database.import_asset_path(&reflection_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Reflect"]);
    assert!(material_metadata.dependencies.contains(&reflection_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&mesh_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&material_id));
    assert!(database
        .registry()
        .get(material_id)
        .unwrap()
        .dependencies
        .contains(&reflection_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert!(loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies
        .contains(&reflection_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(reflection_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_cube_back_projection",
            vec![mesh_id, material_id, reflection_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&reflection_id));
    assert_eq!(
        reader.manifest().dependencies(reflection_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "reflection");
    assert_eq!(material.textures[0].texture.id(), reflection_id);
    assert_eq!(
        material.textures[0].options.projection,
        Some(MaterialTextureProjection::CubeBack)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_cube_left_projection() {
    let config = database_config("builtin_model_obj_cube_left_projection");
    let model_path = AssetPath::parse("models/cube_left.obj");
    let mesh_path = AssetPath::parse("models/cube_left.Panel.mesh");
    let material_path = AssetPath::parse("models/cube_left.Material_Reflect.material");
    let reflection_path = AssetPath::parse("models/textures/cube_left_reflection.texture");
    let model_source = b"mtllib cube_left.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Reflect
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Reflect
map_refl -type cube_left textures/cube_left_reflection.texture
"
    .to_vec();
    let expected_material = b"# mtllib cube_left.mtl
name=Reflect
texture.reflection=models/textures/cube_left_reflection.texture
texture.reflection.projection=cube_left
"
    .to_vec();
    let reflection_source = texture_bytes(1, 1, 118);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/cube_left.mtl", material_source);
    io.insert(reflection_path.path(), reflection_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let reflection_id = database.import_asset_path(&reflection_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Reflect"]);
    assert!(material_metadata.dependencies.contains(&reflection_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&mesh_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&material_id));
    assert!(database
        .registry()
        .get(material_id)
        .unwrap()
        .dependencies
        .contains(&reflection_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert!(loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies
        .contains(&reflection_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(reflection_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_cube_left_projection",
            vec![mesh_id, material_id, reflection_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&reflection_id));
    assert_eq!(
        reader.manifest().dependencies(reflection_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "reflection");
    assert_eq!(material.textures[0].texture.id(), reflection_id);
    assert_eq!(
        material.textures[0].options.projection,
        Some(MaterialTextureProjection::CubeLeft)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_cube_right_projection() {
    let config = database_config("builtin_model_obj_cube_right_projection");
    let model_path = AssetPath::parse("models/cube_right.obj");
    let mesh_path = AssetPath::parse("models/cube_right.Panel.mesh");
    let material_path = AssetPath::parse("models/cube_right.Material_Reflect.material");
    let reflection_path = AssetPath::parse("models/textures/cube_right_reflection.texture");
    let model_source = b"mtllib cube_right.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Reflect
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Reflect
map_refl -type cube_right textures/cube_right_reflection.texture
"
    .to_vec();
    let expected_material = b"# mtllib cube_right.mtl
name=Reflect
texture.reflection=models/textures/cube_right_reflection.texture
texture.reflection.projection=cube_right
"
    .to_vec();
    let reflection_source = texture_bytes(1, 1, 119);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/cube_right.mtl", material_source);
    io.insert(reflection_path.path(), reflection_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let reflection_id = database.import_asset_path(&reflection_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Reflect"]);
    assert!(material_metadata.dependencies.contains(&reflection_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&mesh_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&material_id));
    assert!(database
        .registry()
        .get(material_id)
        .unwrap()
        .dependencies
        .contains(&reflection_id));
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert!(loaded_sidecars
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .dependencies
        .contains(&reflection_id));

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(reflection_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_cube_right_projection",
            vec![mesh_id, material_id, reflection_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    let bundle_material_dependencies = reader.manifest().dependencies(material_id).unwrap();
    assert!(bundle_material_dependencies.contains(&reflection_id));
    assert_eq!(
        reader.manifest().dependencies(reflection_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures[0].name, "reflection");
    assert_eq!(material.textures[0].texture.id(), reflection_id);
    assert_eq!(
        material.textures[0].options.projection,
        Some(MaterialTextureProjection::CubeRight)
    );
}

#[test]
fn database_model_importer_splits_obj_meshes_by_material_assignment() {
    let config = database_config("builtin_model_obj_material_split");
    let model_path = AssetPath::parse("models/multi.obj");
    let red_mesh_path = AssetPath::parse("models/multi.Panel_Material_Red.mesh");
    let blue_mesh_path = AssetPath::parse("models/multi.Panel_Material_Blue.mesh");
    let red_material_path = AssetPath::parse("models/multi.Material_Red.material");
    let blue_material_path = AssetPath::parse("models/multi.Material_Blue.material");
    let model_source = b"mtllib multi.mtl
o Panel
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
usemtl Red
f 1 2 3
usemtl Blue
f 1 3 4
"
    .to_vec();
    let material_source = b"newmtl Red
Kd 1 0 0
newmtl Blue
Kd 0 0 1
"
    .to_vec();
    let expected_red_mesh = b"v 0 0 0
v 1 0 0
v 1 1 0
i 0 1 2
"
    .to_vec();
    let expected_blue_mesh = b"v 0 0 0
v 1 1 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/multi.mtl", material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let red_mesh = database
        .registry()
        .metadata_by_path(&red_mesh_path)
        .unwrap();
    let blue_mesh = database
        .registry()
        .metadata_by_path(&blue_mesh_path)
        .unwrap();
    let red_material = database
        .registry()
        .metadata_by_path(&red_material_path)
        .unwrap();
    let blue_material = database
        .registry()
        .metadata_by_path(&blue_material_path)
        .unwrap();
    let red_mesh_id = red_mesh.id;
    let blue_mesh_id = blue_mesh.id;
    let red_material_id = red_material.id;
    let blue_material_id = blue_material.id;

    assert_eq!(red_mesh.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(blue_mesh.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(red_mesh.labels, vec!["Panel.Material/Red"]);
    assert_eq!(blue_mesh.labels, vec!["Panel.Material/Blue"]);
    assert_eq!(red_mesh.dependencies, vec![red_material_id]);
    assert_eq!(blue_mesh.dependencies, vec![blue_material_id]);
    assert_eq!(
        fs::read(config.imported_root.join(red_mesh_path.path())).unwrap(),
        expected_red_mesh
    );
    assert_eq!(
        fs::read(config.imported_root.join(blue_mesh_path.path())).unwrap(),
        expected_blue_mesh
    );
    assert_eq!(
        fs::read(config.imported_root.join(red_material_path.path())).unwrap(),
        b"# mtllib multi.mtl\nname=Red\nbase_color=1,0,0,1\n".to_vec()
    );
    assert_eq!(
        fs::read(config.imported_root.join(blue_material_path.path())).unwrap(),
        b"# mtllib multi.mtl\nname=Blue\nbase_color=0,0,1,1\n".to_vec()
    );
    let root_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert_eq!(root_dependencies.len(), 4);
    for dependency in [red_mesh_id, blue_mesh_id, red_material_id, blue_material_id] {
        assert!(root_dependencies.contains(&dependency));
    }

    database
        .cook_asset(red_mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(blue_mesh_id, TargetPlatform::Windows)
        .unwrap();
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_usemtl_across_object_groups() {
    let config = database_config("builtin_model_obj_persistent_material_binding");
    let model_path = AssetPath::parse("models/persistent.obj");
    let first_mesh_path = AssetPath::parse("models/persistent.PanelA.mesh");
    let second_mesh_path = AssetPath::parse("models/persistent.PanelB.mesh");
    let material_path = AssetPath::parse("models/persistent.Material_Red.material");
    let model_source = b"mtllib persistent.mtl
usemtl Red
o PanelA
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
g PanelB
v 0 0 1
v 1 0 1
v 0 1 1
f 4 5 6
"
    .to_vec();
    let material_source = b"newmtl Red
Kd 1 0 0
"
    .to_vec();
    let expected_first_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let expected_second_mesh = b"v 0 0 1
v 1 0 1
v 0 1 1
i 0 1 2
"
    .to_vec();
    let expected_material = b"# mtllib persistent.mtl
name=Red
base_color=1,0,0,1
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/persistent.mtl", material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let first_mesh = database
        .registry()
        .metadata_by_path(&first_mesh_path)
        .unwrap();
    let second_mesh = database
        .registry()
        .metadata_by_path(&second_mesh_path)
        .unwrap();
    let material = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let first_mesh_id = first_mesh.id;
    let second_mesh_id = second_mesh.id;
    let material_id = material.id;

    assert_eq!(first_mesh.labels, vec!["PanelA"]);
    assert_eq!(second_mesh.labels, vec!["PanelB"]);
    assert_eq!(material.labels, vec!["Material/Red"]);
    assert_eq!(first_mesh.dependencies, vec![material_id]);
    assert_eq!(second_mesh.dependencies, vec![material_id]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![first_mesh_id, second_mesh_id, material_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(first_mesh_path.path())).unwrap(),
        expected_first_mesh
    );
    assert_eq!(
        fs::read(config.imported_root.join(second_mesh_path.path())).unwrap(),
        expected_second_mesh
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&first_mesh_path)
            .unwrap()
            .dependencies,
        vec![material_id]
    );
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&second_mesh_path)
            .unwrap()
            .dependencies,
        vec![material_id]
    );

    let first_mesh_output = database
        .cook_asset(first_mesh_id, TargetPlatform::Windows)
        .unwrap();
    let second_mesh_output = database
        .cook_asset(second_mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "persistent_material_binding",
            vec![first_mesh_id, second_mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(first_mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(second_mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([].as_slice())
    );
    assert_eq!(
        reader.read_path(&first_mesh_path).unwrap(),
        first_mesh_output.bytes
    );
    assert_eq!(
        reader.read_path(&second_mesh_path).unwrap(),
        second_mesh_output.bytes
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(first_mesh_id),
        &[material_id]
    );
    assert_eq!(
        server
            .dependency_graph()
            .direct_dependencies(second_mesh_id),
        &[material_id]
    );
    assert!(server.get_by_id::<Mesh>(first_mesh_id).is_some());
    assert!(server.get_by_id::<Mesh>(second_mesh_id).is_some());
    assert!(server.get_by_id::<Material>(material_id).is_some());
}

#[test]
fn database_model_importer_combines_obj_multi_group_labels() {
    let config = database_config("builtin_model_obj_multi_group_labels");
    let model_path = AssetPath::parse("models/multi_group.obj");
    let mesh_path = AssetPath::parse("models/multi_group.Body_Shell.mesh");
    let model_source = b"v 0 0 0
v 1 0 0
v 0 1 0
g Body Shell
f 1 2 3
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Body.Shell"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(mesh_metadata.dependencies, Vec::<AssetId>::new());
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
}

#[test]
fn database_model_importer_includes_obj_call_sources() {
    let config = database_config("builtin_model_obj_call_sources");
    let model_path = AssetPath::parse("models/assembled.obj");
    let include_path = AssetPath::parse("models/parts/panel.obj");
    let material_library_path = AssetPath::parse("models/parts/panel.mtl");
    let mesh_path = AssetPath::parse("models/assembled.Panel.mesh");
    let material_path = AssetPath::parse("models/assembled.Material_Blue.material");
    let model_source = b"call parts/panel.obj
"
    .to_vec();
    let first_include = b"mtllib panel.mtl
usemtl Blue
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
"
    .to_vec();
    let second_include = b"mtllib panel.mtl
usemtl Blue
o Panel
v 0 0 0
v 2 0 0
v 0 1 0
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Blue
Kd 0 0 1
"
    .to_vec();
    let first_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let second_mesh = b"v 0 0 0
v 2 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let expected_material = b"# mtllib panel.mtl
name=Blue
base_color=0,0,1,1
"
    .to_vec();
    let mut first_io = MemoryAssetIo::new();
    first_io.insert(model_path.path(), model_source.clone());
    first_io.insert(include_path.path(), first_include);
    first_io.insert(material_library_path.path(), material_source.clone());
    let mut first = AssetDatabase::new(config.clone());
    first.set_io(first_io);
    first.register_builtin_importers();

    let model_id = first.import_asset_path(&model_path).unwrap();
    let first_hash = first.registry().get(model_id).unwrap().source_hash.unwrap();
    let mesh_metadata = first.registry().metadata_by_path(&mesh_path).unwrap();
    let material_metadata = first.registry().metadata_by_path(&material_path).unwrap();
    let mesh_id = mesh_metadata.id;
    let material_id = material_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Panel"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(mesh_metadata.dependencies, vec![material_id]);
    assert_eq!(material_metadata.labels, vec!["Material/Blue"]);
    assert_eq!(
        first.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, material_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        first_mesh
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    first.save_all_metadata_sidecars().unwrap();

    let mut second_io = MemoryAssetIo::new();
    second_io.insert(model_path.path(), model_source);
    second_io.insert(include_path.path(), second_include);
    second_io.insert(material_library_path.path(), material_source);
    let mut second = AssetDatabase::new(config.clone());
    second.set_io(second_io);
    second.register_builtin_importers();

    let report = second.scan_with_metadata().unwrap();

    assert_eq!(report.changed, vec![model_path.clone()]);
    assert!(report.added.contains(&include_path));
    assert!(report.added.contains(&material_library_path));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            AssetDatabaseDiagnostic::ChangedSource {
                id,
                path,
                previous_hash,
                current_hash,
            } if *id == model_id
                && *path == model_path
                && *previous_hash == first_hash
                && previous_hash != current_hash
        )
    }));

    let reimported_id = second.import_asset_path(&model_path).unwrap();
    assert_eq!(reimported_id, model_id);
    assert_ne!(
        second
            .registry()
            .get(reimported_id)
            .unwrap()
            .source_hash
            .unwrap(),
        first_hash
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        second_mesh
    );
}

#[test]
fn database_model_importer_includes_nested_obj_call_sources() {
    let config = database_config("builtin_model_obj_nested_call_sources");
    let model_path = AssetPath::parse("models/assembled.obj");
    let include_path = AssetPath::parse("models/parts/assembly.obj");
    let nested_include_path = AssetPath::parse("models/parts/detail.obj");
    let material_library_path = AssetPath::parse("models/parts/assembly.mtl");
    let mesh_path = AssetPath::parse("models/assembled.Panel.mesh");
    let material_path = AssetPath::parse("models/assembled.Material_Blue.material");
    let model_source = b"call parts/assembly.obj
"
    .to_vec();
    let include_source = b"mtllib assembly.mtl
usemtl Blue
call detail.obj
"
    .to_vec();
    let nested_include_source = b"o Panel
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Blue
Kd 0 0 1
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let expected_material = b"# mtllib assembly.mtl
name=Blue
base_color=0,0,1,1
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert(include_path.path(), include_source);
    io.insert(nested_include_path.path(), nested_include_source);
    io.insert(material_library_path.path(), material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let mesh_id = mesh_metadata.id;
    let material_id = material_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Panel"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(mesh_metadata.dependencies, vec![material_id]);
    assert_eq!(material_metadata.labels, vec!["Material/Blue"]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, material_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    database.save_all_metadata_sidecars().unwrap();

    let mut changed_io = MemoryAssetIo::new();
    changed_io.insert(model_path.path(), b"call parts/assembly.obj\n".to_vec());
    changed_io.insert(
        include_path.path(),
        b"mtllib assembly.mtl\nusemtl Blue\ncall detail.obj\n".to_vec(),
    );
    changed_io.insert(
        nested_include_path.path(),
        b"o Panel\nv 0 0 0\nv 2 0 0\nv 0 1 0\nf 1 2 3\n".to_vec(),
    );
    changed_io.insert(
        material_library_path.path(),
        b"newmtl Blue\nKd 0 0 1\n".to_vec(),
    );
    let mut changed_database = AssetDatabase::new(config.clone());
    changed_database.set_io(changed_io);
    changed_database.register_builtin_importers();

    let report = changed_database.scan_with_metadata().unwrap();
    assert_eq!(report.changed, vec![model_path.clone()]);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            AssetDatabaseDiagnostic::ChangedSource {
                id,
                path,
                ..
            } if *id == model_id && *path == model_path
        )
    }));

    let mut mtl_changed_io = MemoryAssetIo::new();
    mtl_changed_io.insert(model_path.path(), b"call parts/assembly.obj\n".to_vec());
    mtl_changed_io.insert(
        include_path.path(),
        b"mtllib assembly.mtl\nusemtl Blue\ncall detail.obj\n".to_vec(),
    );
    mtl_changed_io.insert(
        nested_include_path.path(),
        b"o Panel\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n".to_vec(),
    );
    mtl_changed_io.insert(
        material_library_path.path(),
        b"newmtl Blue\nKd 0 1 0\n".to_vec(),
    );
    let mut mtl_changed_database = AssetDatabase::new(config.clone());
    mtl_changed_database.set_io(mtl_changed_io);
    mtl_changed_database.register_builtin_importers();

    let mtl_report = mtl_changed_database.scan_with_metadata().unwrap();
    assert_eq!(mtl_report.changed, vec![model_path.clone()]);
    assert!(mtl_report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            AssetDatabaseDiagnostic::ChangedSource {
                id,
                path,
                ..
            } if *id == model_id && *path == model_path
        )
    }));

    let mut non_utf8_io = MemoryAssetIo::new();
    non_utf8_io.insert(model_path.path(), b"call parts/assembly.obj\n".to_vec());
    non_utf8_io.insert(
        include_path.path(),
        b"mtllib assembly.mtl\nusemtl Blue\ncall detail.obj\n".to_vec(),
    );
    non_utf8_io.insert(nested_include_path.path(), vec![0xff, 0xfe, 0xfd, 0xfc]);
    non_utf8_io.insert(
        material_library_path.path(),
        b"newmtl Blue\nKd 0 0 1\n".to_vec(),
    );
    let mut non_utf8_database = AssetDatabase::new(config.clone());
    non_utf8_database.set_io(non_utf8_io);
    non_utf8_database.register_builtin_importers();

    let non_utf8_report = non_utf8_database.scan_with_metadata().unwrap();
    assert_eq!(non_utf8_report.changed, vec![model_path.clone()]);
    assert!(non_utf8_report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            AssetDatabaseDiagnostic::ChangedSource {
                id,
                path,
                ..
            } if *id == model_id && *path == model_path
        )
    }));
}

#[test]
fn database_model_importer_includes_nested_obj_call_relative_material_libraries() {
    let config = database_config("builtin_model_obj_nested_call_relative_material_libraries");
    let model_path = AssetPath::parse("models/assembled.obj");
    let include_path = AssetPath::parse("models/parts/assembly.obj");
    let nested_include_path = AssetPath::parse("models/parts/sub/detail.obj");
    let nested_material_library_path = AssetPath::parse("models/parts/sub/detail.mtl");
    let nested_texture_path = AssetPath::parse("models/parts/sub/textures/detail_albedo.texture");
    let nested_texture_alt_path =
        AssetPath::parse("models/parts/sub/textures/detail_albedo_alt.texture");
    let mesh_path = AssetPath::parse("models/assembled.Panel.mesh");
    let nested_material_path = AssetPath::parse("models/assembled.Material_Detail.material");
    let model_source = b"call parts/assembly.obj
"
    .to_vec();
    let include_source = b"call sub/detail.obj
"
    .to_vec();
    let nested_include_source = b"mtllib detail.mtl
usemtl Detail
o Panel
v 0 0 0
    v 1 0 0
    v 0 1 0
    f 1 2 3
"
    .to_vec();
    let nested_material_source = b"newmtl Detail
map_Kd textures/detail_albedo.texture
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let expected_nested_material = b"# mtllib detail.mtl
name=Detail
texture.albedo=models/parts/sub/textures/detail_albedo.texture
"
    .to_vec();
    let nested_texture_source = texture_bytes(1, 1, 77);
    let nested_texture_alt_source = texture_bytes(1, 1, 123);
    let mut first_io = MemoryAssetIo::new();
    first_io.insert(model_path.path(), model_source.clone());
    first_io.insert(include_path.path(), include_source.clone());
    first_io.insert(nested_include_path.path(), nested_include_source.clone());
    first_io.insert(
        nested_material_library_path.path(),
        nested_material_source.clone(),
    );
    first_io.insert(nested_texture_path.path(), nested_texture_source.clone());
    let mut first = AssetDatabase::new(config.clone());
    first.set_io(first_io);
    first.register_builtin_importers();
    first.register_builtin_cookers();

    let nested_texture_id = first.import_asset_path(&nested_texture_path).unwrap();
    let model_id = first.import_asset_path(&model_path).unwrap();
    let first_hash = first.registry().get(model_id).unwrap().source_hash.unwrap();
    let mesh_metadata = first.registry().metadata_by_path(&mesh_path).unwrap();
    let nested_material_metadata = first
        .registry()
        .metadata_by_path(&nested_material_path)
        .unwrap();
    let nested_texture_metadata = first
        .registry()
        .metadata_by_path(&nested_texture_path)
        .unwrap();
    let mesh_id = mesh_metadata.id;
    let nested_material_id = nested_material_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Panel"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(mesh_metadata.dependencies, vec![nested_material_id]);
    assert_eq!(nested_material_metadata.labels, vec!["Material/Detail"]);
    assert_eq!(
        nested_material_metadata.dependencies,
        vec![nested_texture_id]
    );
    assert_eq!(
        nested_texture_metadata.asset_type,
        AssetTypeId::of::<Texture>()
    );
    assert_eq!(nested_texture_metadata.labels, Vec::<String>::new());
    assert_eq!(
        first.registry().get(model_id).unwrap().dependencies.len(),
        3
    );
    assert!(first
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&mesh_id));
    assert!(first
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&nested_material_id));
    assert!(first
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&nested_texture_id));
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    assert_eq!(
        fs::read(config.imported_root.join(nested_material_path.path())).unwrap(),
        expected_nested_material
    );
    assert_eq!(
        fs::read(config.imported_root.join(nested_texture_path.path())).unwrap(),
        nested_texture_source
    );
    first.save_all_metadata_sidecars().unwrap();

    let mut second_io = MemoryAssetIo::new();
    second_io.insert(model_path.path(), model_source);
    second_io.insert(include_path.path(), include_source);
    second_io.insert(nested_include_path.path(), nested_include_source);
    second_io.insert(
        nested_material_library_path.path(),
        b"newmtl Detail\nmap_Kd textures/detail_albedo_alt.texture\n".to_vec(),
    );
    second_io.insert(
        nested_texture_alt_path.path(),
        nested_texture_alt_source.clone(),
    );
    let mut second = AssetDatabase::new(config.clone());
    second.set_io(second_io);
    second.register_builtin_importers();
    second.register_builtin_cookers();

    let second_nested_texture_id = second.import_asset_path(&nested_texture_alt_path).unwrap();
    let report = second.scan_with_metadata().unwrap();
    assert!(report.changed.contains(&model_path));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic,
            AssetDatabaseDiagnostic::ChangedSource {
                id,
                path,
                previous_hash,
                current_hash,
            } if *id == model_id
                && *path == model_path
                && *previous_hash == first_hash
                && previous_hash != current_hash
        )
    }));

    let reimported_id = second.import_asset_path(&model_path).unwrap();
    assert_eq!(reimported_id, model_id);
    let nested_material_metadata = second
        .registry()
        .metadata_by_path(&nested_material_path)
        .unwrap();
    assert_eq!(nested_material_metadata.dependencies.len(), 1);
    assert_eq!(
        nested_material_metadata.dependencies[0],
        second_nested_texture_id
    );
}

#[test]
fn database_model_importer_reports_invalid_nested_obj_material_texture_path() {
    let config = database_config("builtin_model_obj_nested_material_texture_escape");
    let model_path = AssetPath::parse("models/assembled.obj");
    let include_path = AssetPath::parse("models/parts/assembly.obj");
    let nested_include_path = AssetPath::parse("models/parts/sub/detail.obj");
    let nested_material_library_path = AssetPath::parse("models/parts/sub/detail.mtl");
    let model_source = b"call parts/assembly.obj
"
    .to_vec();
    let include_source = b"call sub/detail.obj
"
    .to_vec();
    let nested_include_source = b"mtllib detail.mtl
usemtl Detail
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
"
    .to_vec();
    let nested_material_source = b"newmtl Detail
map_Kd ../escape.texture
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert(include_path.path(), include_source);
    io.insert(nested_include_path.path(), nested_include_source);
    io.insert(nested_material_library_path.path(), nested_material_source);
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/assembled.obj")
                && message.contains(
                    "OBJ material library `detail.mtl` at `models/parts/sub/detail.mtl` map_Kd texture path `../escape.texture` on line 2 must be a relative source path without labels or `..` segments"
                )
    ));
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_accepts_nested_obj_quoted_marker_paths() {
    let config = database_config("builtin_model_obj_nested_quoted_marker_paths");
    let model_path = AssetPath::parse("models/assembled.obj");
    let include_path = AssetPath::parse("models/parts/assembly.obj");
    let nested_include_path = AssetPath::parse("models/parts/sub/detail.obj");
    let nested_material_library_path = AssetPath::parse("models/parts/sub/detail.mtl");
    let mesh_path = AssetPath::parse("models/assembled.Panel.mesh");
    let nested_material_path = AssetPath::parse("models/assembled.Material_Detail.material");
    let model_source = b"call parts/assembly.obj
"
    .to_vec();
    let include_source = b"call sub/detail.obj
"
    .to_vec();
    let nested_include_source = b"mtllib \"detail.mtl\"
usemtl Detail
MAPLIB \"procedural maps/detail.map\"
UseMap \"Checker Map\"
SHADOW_OBJ \"shadows/soft shadow.obj\"
trace_obj \"rays/primary ray.obj\"
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
"
    .to_vec();
    let nested_material_source = b"newmtl Detail
Kd 0.25 0.5 0.75
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let expected_nested_material = b"# mtllib detail.mtl
name=Detail
base_color=0.25,0.5,0.75,1
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert(include_path.path(), include_source);
    io.insert(nested_include_path.path(), nested_include_source);
    io.insert(nested_material_library_path.path(), nested_material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let nested_material_metadata = database
        .registry()
        .metadata_by_path(&nested_material_path)
        .unwrap();
    let mesh_id = mesh_metadata.id;
    let nested_material_id = nested_material_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Panel"]);
    assert_eq!(nested_material_metadata.labels, vec!["Material/Detail"]);
    assert_eq!(
        database
            .registry()
            .get(model_id)
            .unwrap()
            .dependencies
            .len(),
        2
    );
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&mesh_id));
    assert!(database
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&nested_material_id));
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    assert_eq!(
        fs::read(config.imported_root.join(nested_material_path.path())).unwrap(),
        expected_nested_material
    );

    let mesh_output = database
        .cook_asset(mesh_metadata.id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "nested_quoted_marker_paths",
            vec![mesh_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([nested_material_id].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);
}

#[test]
fn database_model_importer_includes_nested_obj_quoted_material_texture_paths() {
    let config = database_config("builtin_model_obj_nested_quoted_material_texture_paths");
    let model_path = AssetPath::parse("models/assembled.obj");
    let include_path = AssetPath::parse("models/parts/assembly.obj");
    let nested_include_path = AssetPath::parse("models/parts/sub/detail.obj");
    let nested_material_library_path = AssetPath::parse("models/parts/sub/detail.mtl");
    let nested_texture_path = AssetPath::parse("models/parts/sub/textures/detail albedo.texture");
    let mesh_path = AssetPath::parse("models/assembled.Panel.mesh");
    let nested_material_path = AssetPath::parse("models/assembled.Material_Detail.material");
    let model_source = b"call parts/assembly.obj
"
    .to_vec();
    let include_source = b"call sub/detail.obj
"
    .to_vec();
    let nested_include_source = b"mtllib detail.mtl
usemtl Detail
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
"
    .to_vec();
    let nested_material_source = b"newmtl Detail
map_Kd \"textures/detail albedo.texture\"
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let expected_nested_material = b"# mtllib detail.mtl
name=Detail
texture.albedo=models/parts/sub/textures/detail albedo.texture
"
    .to_vec();
    let nested_texture_source = texture_bytes(1, 1, 77);
    let mut first_io = MemoryAssetIo::new();
    first_io.insert(model_path.path(), model_source.clone());
    first_io.insert(include_path.path(), include_source.clone());
    first_io.insert(nested_include_path.path(), nested_include_source.clone());
    first_io.insert(
        nested_material_library_path.path(),
        nested_material_source.clone(),
    );
    first_io.insert(nested_texture_path.path(), nested_texture_source.clone());
    let mut first = AssetDatabase::new(config.clone());
    first.set_io(first_io);
    first.register_builtin_importers();
    first.register_builtin_cookers();

    let nested_texture_id = first.import_asset_path(&nested_texture_path).unwrap();
    let model_id = first.import_asset_path(&model_path).unwrap();
    let mesh_metadata = first.registry().metadata_by_path(&mesh_path).unwrap();
    let nested_material_metadata = first
        .registry()
        .metadata_by_path(&nested_material_path)
        .unwrap();

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Panel"]);
    assert_eq!(mesh_metadata.dependencies.len(), 1);
    assert_eq!(nested_material_metadata.labels, vec!["Material/Detail"]);
    assert_eq!(
        nested_material_metadata.dependencies,
        vec![nested_texture_id]
    );
    assert_eq!(
        first.registry().get(model_id).unwrap().dependencies.len(),
        3
    );
    assert!(first
        .registry()
        .get(model_id)
        .unwrap()
        .dependencies
        .contains(&nested_texture_id));
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    assert_eq!(
        fs::read(config.imported_root.join(nested_material_path.path())).unwrap(),
        expected_nested_material
    );
    assert_eq!(
        fs::read(config.imported_root.join(nested_texture_path.path())).unwrap(),
        nested_texture_source
    );
    first.save_all_metadata_sidecars().unwrap();

    let mut second_io = MemoryAssetIo::new();
    second_io.insert(model_path.path(), model_source);
    second_io.insert(include_path.path(), include_source);
    second_io.insert(nested_include_path.path(), nested_include_source);
    second_io.insert(
        nested_material_library_path.path(),
        b"newmtl Detail\nmap_Kd \"textures/detail albedo.texture\"\n".to_vec(),
    );
    second_io.insert(nested_texture_path.path(), texture_bytes(1, 1, 123));
    let mut second = AssetDatabase::new(config.clone());
    second.set_io(second_io);
    second.register_builtin_importers();
    second.register_builtin_cookers();

    let second_nested_texture_id = second.import_asset_path(&nested_texture_path).unwrap();
    let _ = second.import_asset_path(&model_path).unwrap();
    let second_nested_material_metadata = second
        .registry()
        .metadata_by_path(&nested_material_path)
        .unwrap();
    assert_eq!(
        second_nested_material_metadata.dependencies,
        vec![second_nested_texture_id]
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_accepts_nested_obj_display_attributes() {
    let config = database_config("builtin_model_obj_nested_display_attributes");
    let model_path = AssetPath::parse("models/assembled.obj");
    let include_path = AssetPath::parse("models/parts/decor.obj");
    let mesh_path = AssetPath::parse("models/assembled.Decor.mesh");
    let model_source = b"call parts/decor.obj
"
    .to_vec();
    let include_source = b"o Decor
MAPLIB procedural.map detail.map
UseMap CheckerMap
usemap OFF
SHADOW_OBJ shadows.obj
trace_obj rays.obj
CTECH cparm 8
ctech curv 0.25 30
STECH cparma 4 6
stech cspace 0.125
stech special
cstype rat bspline
deg 3 2
bmat U 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1
step 4 5
parm V 0 0.5 1
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert(include_path.path(), include_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Decor"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "nested_display_attributes",
            vec![mesh_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(reader.manifest().dependencies(mesh_id), Some([].as_slice()));
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);
}

#[test]
fn database_model_importer_reports_invalid_obj_call_sources() {
    for (case, source, expected) in [
        (
            "builtin_model_obj_empty_call",
            "call\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ call is empty on line 1",
        ),
        (
            "builtin_model_obj_call_with_macro_args",
            "call part.obj 1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ call on line 1 accepts exactly one relative .obj source path; macro arguments are unsupported",
        ),
        (
            "builtin_model_obj_call_non_obj_source",
            "call part.model\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ call `part.model` on line 1 must reference a .obj source",
        ),
        (
            "builtin_model_obj_call_manifest_source",
            "call part.obj\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ call `part.obj` on line 1 source `models/part.obj` must be an OBJ source, not an NGA_MODEL_V1 manifest",
        ),
        (
            "builtin_model_obj_call_non_utf8_source",
            "call part.obj\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ call `part.obj` on line 1 source `models/part.obj` must be UTF-8",
        ),
        (
            "builtin_model_obj_call_parent_path",
            "call ../part.obj\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ call `../part.obj` on line 1 must be a relative .obj source path without labels or `..` segments",
        ),
        (
            "builtin_model_obj_call_missing_source",
            "call missing.obj\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ call `missing.obj` on line 1 could not find source `models/missing.obj`",
        ),
    ] {
        let config = database_config(case);
        let model_path = AssetPath::parse(&format!("models/{case}.obj"));
        let mut io = MemoryAssetIo::new();
        io.insert(model_path.path(), source.as_bytes().to_vec());
        if case == "builtin_model_obj_call_manifest_source" {
            io.insert("models/part.obj", b"NGA_MODEL_V1\n".to_vec());
        } else if case == "builtin_model_obj_call_non_utf8_source" {
            io.insert("models/part.obj", vec![0xff, 0xfe, 0xfd]);
        }
        let mut database = AssetDatabase::new(config);
        database.set_io(io);
        database.register_builtin_importers();

        let error = database.import_asset_path(&model_path).unwrap_err();

        assert!(matches!(
            error,
            AssetError::Import { message }
                if message.contains("importer `ModelImporter` failed")
                    && message.contains(&model_path.display_string())
                    && message.contains(expected)
        ));
    }

    let config = database_config("builtin_model_obj_call_cycle");
    let model_path = AssetPath::parse("models/recursive.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), b"call part.obj\n".to_vec());
    io.insert("models/part.obj", b"call loop.obj\n".to_vec());
    io.insert("models/loop.obj", b"call part.obj\n".to_vec());
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/recursive.obj")
                && message.contains("OBJ call `part.obj` on line 1 failed for `models/part.obj`")
                && message.contains("OBJ call `loop.obj` on line 1 failed for `models/loop.obj`")
                && message.contains(
                    "OBJ call `part.obj` on line 1 would recursively include `models/part.obj`"
                )
    ));
}

#[test]
fn database_model_importer_reports_obj_usemtl_missing_from_loaded_mtl() {
    let config = database_config("builtin_model_obj_missing_loaded_material");
    let model_path = AssetPath::parse("models/missing_loaded_material.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib palette.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Blue
f 1 2 3
"
        .to_vec(),
    );
    io.insert("models/palette.mtl", b"newmtl Red\nKd 1 0 0\n".to_vec());
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/missing_loaded_material.obj")
                && message.contains(
                    "OBJ usemtl `Blue` on line 5 is not defined by loaded mtllib source(s): palette.mtl"
                )
    ));

    let fallback_config = database_config("builtin_model_obj_missing_mtl_fallback");
    let model_path = AssetPath::parse("models/missing_mtl_fallback.obj");
    let material_path = AssetPath::parse("models/missing_mtl_fallback.Material_Ghost.material");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib unavailable.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Ghost
f 1 2 3
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(fallback_config.clone());
    database.set_io(io);
    database.register_builtin_importers();

    database.import_asset_path(&model_path).unwrap();

    assert_eq!(
        fs::read(fallback_config.imported_root.join(material_path.path())).unwrap(),
        b"# mtllib unavailable.mtl\nname=Ghost\n".to_vec()
    );
}

#[test]
fn database_model_importer_parses_obj_relative_indices() {
    let config = database_config("builtin_model_obj_relative_indices");
    let model_path = AssetPath::parse("models/relative.obj");
    let mesh_path = AssetPath::parse("models/relative.Relative.mesh");
    let model_source = b"o Relative
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
vt 0 0
vt 1 0
vt 1 1
vt 0 1
vn 0 0 1
f -4/-4/-1 -3/-3/-1 -2/-2/-1 -1/-1/-1
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
n 0 0 1
n 0 0 1
n 0 0 1
n 0 0 1
uv 0 0
uv 1 0
uv 1 1
uv 0 1
t 1 0 0 1
t 1 0 0 1
t 1 0 0 1
t 1 0 0 1
i 0 1 2
i 0 2 3
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Relative"]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    assert_eq!(mesh_output.bytes, obj_binary_mesh_bytes());
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_accepts_obj_face_outline_alias() {
    let config = database_config("builtin_model_obj_face_outline_alias");
    let model_path = AssetPath::parse("models/face_outline.obj");
    let mesh_path = AssetPath::parse("models/face_outline.Outline.mesh");
    let model_source = b"o Outline
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
Fo 1 2 3 4
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
i 0 1 2
i 0 2 3
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Outline"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "face_outline",
            vec![mesh_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(reader.manifest().dependencies(mesh_id), Some([].as_slice()));
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(
        mesh.vertices,
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0]
        ]
    );
    assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_parses_obj_line_continuations() {
    let config = database_config("builtin_model_obj_line_continuations");
    let model_path = AssetPath::parse("models/continued.obj");
    let mesh_path = AssetPath::parse("models/continued.Continued.mesh");
    let material_path = AssetPath::parse("models/continued.Material_Red.material");
    let model_source = br#"mtllib continued.mtl
o Continued
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
usemtl Red
f 1 2 \
  3 4
"#
    .to_vec();
    let material_source = br#"newmtl Red
Kd 0.25 \
  0.5 0.75
"#
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
i 0 1 2
i 0 2 3
"
    .to_vec();
    let expected_material = b"# mtllib continued.mtl
name=Red
base_color=0.25,0.5,0.75,1
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/continued.mtl", material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let mesh_id = mesh_metadata.id;
    let material_id = material_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(material_metadata.asset_type, AssetTypeId::of::<Material>());
    assert_eq!(mesh_metadata.labels, vec!["Continued"]);
    assert_eq!(material_metadata.labels, vec!["Material/Red"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(material_metadata.importer_version, 111);
    assert_eq!(mesh_metadata.dependencies, vec![material_id]);
    assert!(material_metadata.dependencies.is_empty());
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id, material_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let material_output = database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "line_continuations",
            vec![mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);
    assert_eq!(
        reader.read_path(&material_path).unwrap(),
        material_output.bytes
    );

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.dependency_graph().direct_dependencies(mesh_id),
        &[material_id]
    );
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.properties.base_color, [0.25, 0.5, 0.75, 1.0]);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_accepts_obj_parameter_vertices() {
    let config = database_config("builtin_model_obj_parameter_vertices");
    let model_path = AssetPath::parse("models/parameter_vertices.obj");
    let mesh_path = AssetPath::parse("models/parameter_vertices.Surface.mesh");
    let model_source = b"o Surface
vp 0.5
vp 0.25 0.5
vp 0.1 0.2 0.3
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Surface"]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "parameter_vertices",
            vec![mesh_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(reader.manifest().dependencies(mesh_id), Some([].as_slice()));
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(
        mesh.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_accepts_obj_point_and_line_elements() {
    let config = database_config("builtin_model_obj_point_and_line_elements");
    let model_path = AssetPath::parse("models/point_line_elements.obj");
    let mesh_path = AssetPath::parse("models/point_line_elements.WireHelpers.mesh");
    let model_source = b"o WireHelpers
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
p 1 -2 3
l 1/1 2/2 -1/3
l -3 -2 -1
f 1 2 3
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["WireHelpers"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "point_line_elements",
            vec![mesh_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(reader.manifest().dependencies(mesh_id), Some([].as_slice()));
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(
        mesh.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_accepts_obj_display_attributes() {
    let config = database_config("builtin_model_obj_display_attributes");
    let model_path = AssetPath::parse("models/display_attributes.obj");
    let mesh_path = AssetPath::parse("models/display_attributes.Display.mesh");
    let model_source = b"o Display
bevel ON
c_interp off
d_interp On
lod 2
mg 7 0.5
mg OFF
MAPLIB procedural.map detail.map
UseMap CheckerMap
usemap OFF
SHADOW_OBJ shadows.obj
trace_obj rays.obj
CTECH cparm 8
ctech curv 0.25 30
STECH cparma 4 6
stech cspace 0.125
stech special
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Display"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "display_attributes",
            vec![mesh_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(reader.manifest().dependencies(mesh_id), Some([].as_slice()));
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(
        mesh.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_accepts_obj_free_form_attributes() {
    let config = database_config("builtin_model_obj_free_form_attributes");
    let model_path = AssetPath::parse("models/free_form.obj");
    let mesh_path = AssetPath::parse("models/free_form.FreeForm.mesh");
    let model_source = b"o FreeForm
CSTYPE rat bspline
deg 3 2
bmat U 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1
step 4 5
vp 0 0
vp 1 0
vp 1 1
vp 0 1
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
vt 0 0
vt 1 0
vt 1 1
vt 0 1
vn 0 0 1
curv 0 1 1 2 3
CURV2 1 2 3
surf 0 1 0 1 1/1/1 2/2/1 3/3/1 4/4/1
surf 0 1 0 1 4/4/1 3/3/1 2/2/1 1/1/1
parm V 0 0.5 1
trim 0 1 1
hole 0 1 1
scrv 0 1 1
con 1 0 1 1 2 0 1 1
sp 1 -1
END
f 1 2 3
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 1 1 0
i 0 1 2
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["FreeForm"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new("free_form", vec![mesh_id]))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(reader.manifest().dependencies(mesh_id), Some([].as_slice()));
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(
        mesh.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]
    );
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_accepts_obj_free_form_relative_references() {
    let config = database_config("builtin_model_obj_free_form_relative_references");
    let model_path = AssetPath::parse("models/free_form_relative.obj");
    let mesh_path = AssetPath::parse("models/free_form_relative.FreeFormRelative.mesh");
    let model_source = b"o FreeFormRelative
vp 0 0
vp 1 0
vp 1 1
vp 0 1
v 0 0 0
v 1 0 0
v 1 1 0
v 0 1 0
vt 0 0
vt 1 0
vt 1 1
vt 0 1
vn 0 0 1
curv 0 1 -4 -3 -2
curv2 -4 -3 -2
surf 0 1 0 1 -4/-4/-1 -3/-3/-1 -2/-2/-1 -1/-1/-1
surf 0 1 0 1 -1/-1/-1 -2/-2/-1 -3/-3/-1 -4/-4/-1
trim 0 1 -1
hole 0 1 -1
scrv 0 1 -1
con -2 0 1 -1 -1 0 1 -1
sp -4 -1
end
f 1 2 3
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 1 1 0
i 0 1 2
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["FreeFormRelative"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "free_form_relative",
            vec![mesh_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(reader.manifest().dependencies(mesh_id), Some([].as_slice()));
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(
        mesh.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]
    );
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_parses_obj_homogeneous_vertices() {
    let config = database_config("builtin_model_obj_homogeneous_vertices");
    let model_path = AssetPath::parse("models/homogeneous.obj");
    let mesh_path = AssetPath::parse("models/homogeneous.Homogeneous.mesh");
    let model_source = b"o Homogeneous
v 2 0 0 2
v 0 4 0 2
v 0 0 6 3
f 1 2 3
"
    .to_vec();
    let expected_mesh = b"v 2 0 0
v 0 4 0
v 0 0 4
i 0 1 2
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let mut settings = ImporterSettings::default();
    settings.set("scale", "2");
    let model_id = database
        .import_asset_path_with_settings(&model_path, &settings)
        .unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Homogeneous"]);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );
    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "homogeneous_vertices",
            vec![mesh_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(reader.manifest().dependencies(mesh_id), Some([].as_slice()));
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(
        mesh.vertices,
        vec![[2.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 4.0]]
    );
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
fn database_model_importer_reports_invalid_obj_parameter_vertex() {
    for (case, source, expected) in [
        (
            "builtin_model_obj_missing_parameter_vertex",
            "vp\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ parameter vertex value on line 1",
        ),
        (
            "builtin_model_obj_non_finite_parameter_vertex",
            "vp 0 NaN\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ parameter vertex value must be finite on line 1",
        ),
        (
            "builtin_model_obj_too_many_parameter_vertex_values",
            "vp 0 0 0 1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "too many OBJ parameter vertex values on line 1",
        ),
    ] {
        let config = database_config(case);
        let model_path = AssetPath::parse(&format!("models/{case}.obj"));
        let mut io = MemoryAssetIo::new();
        io.insert(model_path.path(), source.as_bytes().to_vec());
        let mut database = AssetDatabase::new(config);
        database.set_io(io);
        database.register_builtin_importers();

        let error = database.import_asset_path(&model_path).unwrap_err();

        assert!(matches!(
            error,
            AssetError::Import { message }
                if message.contains("importer `ModelImporter` failed")
                    && message.contains(&model_path.display_string())
                    && message.contains(expected)
        ));
    }
}

#[test]
fn database_model_importer_reports_invalid_obj_point_and_line_elements() {
    for (case, source, expected) in [
        (
            "builtin_model_obj_missing_point_vertex",
            "p\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ point on line 1 must contain at least 1 vertex",
        ),
        (
            "builtin_model_obj_missing_point_vertex_reference",
            "v 0 0 0\np 2\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ point index 2 on line 2 references missing vertex",
        ),
        (
            "builtin_model_obj_short_line_element",
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nl 1\nf 1 2 3\n",
            "OBJ line on line 4 must contain at least 2 vertices",
        ),
        (
            "builtin_model_obj_missing_line_texture_coordinate",
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nl 1/1 2/2\nf 1 2 3\n",
            "OBJ line texture coordinate index 2 on line 5 references missing texture coordinate",
        ),
        (
            "builtin_model_obj_invalid_line_tuple",
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nl 1//1 2//1\nf 1 2 3\n",
            "invalid OBJ line tuple `1//1` on line 4",
        ),
    ] {
        let config = database_config(case);
        let model_path = AssetPath::parse(&format!("models/{case}.obj"));
        let mut io = MemoryAssetIo::new();
        io.insert(model_path.path(), source.as_bytes().to_vec());
        let mut database = AssetDatabase::new(config);
        database.set_io(io);
        database.register_builtin_importers();

        let error = database.import_asset_path(&model_path).unwrap_err();

        assert!(matches!(
            error,
            AssetError::Import { message }
                if message.contains("importer `ModelImporter` failed")
                    && message.contains(&model_path.display_string())
                    && message.contains(expected)
        ));
    }
}

#[test]
fn database_model_importer_reports_invalid_obj_display_attributes() {
    for (case, source, expected) in [
        (
            "builtin_model_obj_invalid_bevel_attribute",
            "bevel maybe\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ bevel value on line 1 must be `on` or `off`",
        ),
        (
            "builtin_model_obj_missing_c_interp_attribute",
            "c_interp\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ c_interp value on line 1",
        ),
        (
            "builtin_model_obj_extra_d_interp_attribute",
            "d_interp off extra\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "too many OBJ d_interp values on line 1",
        ),
        (
            "builtin_model_obj_negative_lod_attribute",
            "lod -1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ lod value on line 1 must be non-negative",
        ),
        (
            "builtin_model_obj_invalid_lod_attribute",
            "lod high\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "invalid OBJ lod value on line 1",
        ),
        (
            "builtin_model_obj_extra_lod_attribute",
            "lod 1 2\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "too many OBJ lod values on line 1",
        ),
        (
            "builtin_model_obj_missing_merging_group",
            "mg\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ merging group value on line 1",
        ),
        (
            "builtin_model_obj_missing_merging_group_resolution",
            "mg 1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ merging group resolution on line 1",
        ),
        (
            "builtin_model_obj_non_positive_merging_group",
            "mg -1 0.5\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ merging group number on line 1 must be positive or `off`",
        ),
        (
            "builtin_model_obj_invalid_merging_group_number",
            "mg high 0.5\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "invalid OBJ merging group number `high` on line 1",
        ),
        (
            "builtin_model_obj_invalid_merging_group_resolution",
            "mg 1 high\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "invalid OBJ merging group resolution on line 1",
        ),
        (
            "builtin_model_obj_non_finite_merging_group_resolution",
            "mg 1 NaN\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ merging group resolution must be finite and non-negative on line 1",
        ),
        (
            "builtin_model_obj_extra_merging_group_values",
            "mg off 0.5\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "too many OBJ merging group values on line 1",
        ),
        (
            "builtin_model_obj_empty_maplib_attribute",
            "maplib\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ maplib is empty on line 1",
        ),
        (
            "builtin_model_obj_invalid_maplib_parent_path",
            "maplib ../procedural.map\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ maplib `../procedural.map` on line 1 must be a relative source path without labels or `..` segments",
        ),
        (
            "builtin_model_obj_invalid_maplib_label_path",
            "maplib \"procedural.map#Variant\"\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ maplib `procedural.map#Variant` on line 1 must be a relative source path without labels or `..` segments",
        ),
        (
            "builtin_model_obj_missing_usemap_attribute",
            "usemap\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ usemap value on line 1",
        ),
        (
            "builtin_model_obj_extra_usemap_attribute",
            "usemap MapA MapB\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "too many OBJ usemap values on line 1",
        ),
        (
            "builtin_model_obj_missing_shadow_object_attribute",
            "shadow_obj\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ shadow_obj value on line 1",
        ),
        (
            "builtin_model_obj_extra_shadow_object_attribute",
            "shadow_obj a.obj b.obj\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "too many OBJ shadow_obj values on line 1",
        ),
        (
            "builtin_model_obj_invalid_shadow_object_path",
            "shadow_obj ../shadows.obj\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ shadow_obj `../shadows.obj` on line 1 must be a relative source path without labels or `..` segments",
        ),
        (
            "builtin_model_obj_missing_trace_object_attribute",
            "trace_obj\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ trace_obj value on line 1",
        ),
        (
            "builtin_model_obj_extra_trace_object_attribute",
            "trace_obj a.obj b.obj\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "too many OBJ trace_obj values on line 1",
        ),
        (
            "builtin_model_obj_invalid_trace_object_path",
            "trace_obj /rays.obj\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ trace_obj `/rays.obj` on line 1 must be a relative source path without labels or `..` segments",
        ),
        (
            "builtin_model_obj_missing_curve_technique",
            "ctech\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ ctech technique on line 1",
        ),
        (
            "builtin_model_obj_unknown_curve_technique",
            "ctech unknown 1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "unknown OBJ ctech technique `unknown` on line 1",
        ),
        (
            "builtin_model_obj_missing_curve_technique_value",
            "ctech cspace\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ ctech cspace value on line 1",
        ),
        (
            "builtin_model_obj_non_finite_curve_technique_value",
            "ctech cparm NaN\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ ctech cparm value must be finite and non-negative on line 1",
        ),
        (
            "builtin_model_obj_extra_curve_technique_value",
            "ctech curv 0.1 30 4\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "too many OBJ ctech curv values on line 1",
        ),
        (
            "builtin_model_obj_missing_surface_technique_value",
            "stech cparma 4\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ stech cparma value on line 1",
        ),
        (
            "builtin_model_obj_negative_surface_technique_value",
            "stech cparmb -1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ stech cparmb value must be finite and non-negative on line 1",
        ),
        (
            "builtin_model_obj_extra_special_surface_technique_value",
            "stech special 1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "too many OBJ stech special values on line 1",
        ),
    ] {
        let config = database_config(case);
        let model_path = AssetPath::parse(&format!("models/{case}.obj"));
        let mut io = MemoryAssetIo::new();
        io.insert(model_path.path(), source.as_bytes().to_vec());
        let mut database = AssetDatabase::new(config);
        database.set_io(io);
        database.register_builtin_importers();

        let error = database.import_asset_path(&model_path).unwrap_err();

        assert!(matches!(
            error,
            AssetError::Import { message }
                if message.contains("importer `ModelImporter` failed")
                    && message.contains(&model_path.display_string())
                    && message.contains(expected)
        ));
    }
}

#[test]
fn database_model_importer_reports_invalid_obj_free_form_attributes() {
    for (case, source, expected) in [
        (
            "builtin_model_obj_missing_cstype",
            "cstype\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ cstype value on line 1",
        ),
        (
            "builtin_model_obj_unknown_cstype",
            "cstype spline\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "unknown OBJ cstype value `spline` on line 1",
        ),
        (
            "builtin_model_obj_non_positive_degree",
            "deg 0\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ deg value on line 1 must be positive",
        ),
        (
            "builtin_model_obj_unknown_basis_matrix_direction",
            "bmat q 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "unknown OBJ bmat direction `q` on line 1",
        ),
        (
            "builtin_model_obj_missing_basis_matrix_value",
            "bmat u 1 0 0\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ bmat value on line 1",
        ),
        (
            "builtin_model_obj_non_positive_step",
            "step 1 -2\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ step value on line 1 must be positive",
        ),
        (
            "builtin_model_obj_short_curve",
            "v 0 0 0\ncurv 0 1 1\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ curv on line 2 must contain at least 2 control points",
        ),
        (
            "builtin_model_obj_missing_curve_vertex",
            "v 0 0 0\ncurv 0 1 1 2\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ curv index 2 on line 2 references missing vertex",
        ),
        (
            "builtin_model_obj_missing_relative_curve_vertex",
            "v 0 0 0\nv 1 0 0\ncurv 0 1 -3 -1\nv 0 1 0\nf 1 2 3\n",
            "OBJ curv index -3 on line 3 references missing vertex",
        ),
        (
            "builtin_model_obj_missing_curve2_parameter_vertex",
            "vp 0 0\ncurv2 1 2\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ curv2 index 2 on line 2 references missing parameter vertex",
        ),
        (
            "builtin_model_obj_short_surface",
            "v 0 0 0\nv 1 0 0\nv 1 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvn 0 0 1\nsurf 0 1 0 1 1/1/1 2/2/1 3/3/1\nf 1 2 3\n",
            "OBJ surf on line 8 must contain at least 4 control points",
        ),
        (
            "builtin_model_obj_unknown_parameter_direction",
            "parm q 0 1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "unknown OBJ parm direction `q` on line 1",
        ),
        (
            "builtin_model_obj_incomplete_trim_group",
            "vp 0 0\nvp 1 0\ncurv2 1 2\ntrim 0 1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ trim on line 4 requires complete `<u0> <u1> <curv2>` groups",
        ),
        (
            "builtin_model_obj_missing_trim_curve2",
            "trim 0 1 1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ trim index 1 on line 1 references missing curve2",
        ),
        (
            "builtin_model_obj_missing_connection_surface",
            "con\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "missing OBJ con surface reference on line 1",
        ),
        (
            "builtin_model_obj_missing_connection_surface_reference",
            "vp 0 0\nvp 1 0\ncurv2 1 2\ncon 1 0 1 1 2 0 1 1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ con index 1 on line 4 references missing surface",
        ),
        (
            "builtin_model_obj_missing_connection_curve2_reference",
            "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nsurf 0 1 0 1 1 2 3 4\nsurf 0 1 0 1 4 3 2 1\ncon 1 0 1 1 2 0 1 1\nf 1 2 3\n",
            "OBJ con index 1 on line 7 references missing curve2",
        ),
        (
            "builtin_model_obj_non_finite_connection_range",
            "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvp 0 0\nvp 1 0\ncurv2 1 2\nsurf 0 1 0 1 1 2 3 4\nsurf 0 1 0 1 4 3 2 1\ncon 1 0 NaN 1 2 0 1 1\nf 1 2 3\n",
            "OBJ con range value must be finite on line 10",
        ),
        (
            "builtin_model_obj_extra_connection_values",
            "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvp 0 0\nvp 1 0\ncurv2 1 2\nsurf 0 1 0 1 1 2 3 4\nsurf 0 1 0 1 4 3 2 1\ncon 1 0 1 1 2 0 1 1 extra\nf 1 2 3\n",
            "too many OBJ con values on line 10",
        ),
        (
            "builtin_model_obj_missing_special_point",
            "sp\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "OBJ sp on line 1 must contain at least 1 special point",
        ),
        (
            "builtin_model_obj_extra_end_value",
            "end now\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "too many OBJ end values on line 1",
        ),
    ] {
        let config = database_config(case);
        let model_path = AssetPath::parse(&format!("models/{case}.obj"));
        let mut io = MemoryAssetIo::new();
        io.insert(model_path.path(), source.as_bytes().to_vec());
        let mut database = AssetDatabase::new(config);
        database.set_io(io);
        database.register_builtin_importers();

        let error = database.import_asset_path(&model_path).unwrap_err();

        assert!(matches!(
            error,
            AssetError::Import { message }
                if message.contains("importer `ModelImporter` failed")
                    && message.contains(&model_path.display_string())
                    && message.contains(expected)
        ));
    }
}

#[test]
fn database_model_importer_reports_invalid_obj_face_index() {
    let relative_config = database_config("builtin_model_obj_invalid_relative_face");
    let model_path = AssetPath::parse("models/bad_relative.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf -4 -2 -1\n".to_vec(),
    );
    let mut database = AssetDatabase::new(relative_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_relative.obj")
                && message.contains("OBJ face index -4 on line 4 references missing vertex")
    ));

    let face_outline_config = database_config("builtin_model_obj_invalid_face_outline_alias");
    let model_path = AssetPath::parse("models/bad_face_outline.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nfo 1 2\n".to_vec(),
    );
    let mut database = AssetDatabase::new(face_outline_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_face_outline.obj")
                && message.contains("OBJ face on line 4 must contain at least 3 vertices")
    ));

    let homogeneous_config = database_config("builtin_model_obj_invalid_homogeneous_vertex");
    let model_path = AssetPath::parse("models/bad_homogeneous.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"v 1 2 3 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n".to_vec(),
    );
    let mut database = AssetDatabase::new(homogeneous_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_homogeneous.obj")
                && message.contains(
                    "OBJ vertex homogeneous coordinate must be finite and non-zero on line 1"
                )
    ));

    let mtllib_path_config = database_config("builtin_model_obj_invalid_mtllib_path");
    let model_path = AssetPath::parse("models/bad_mtllib.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib ../bad.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl Red\nf 1 2 3\n".to_vec(),
    );
    let mut database = AssetDatabase::new(mtllib_path_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_mtllib.obj")
                && message.contains(
                    "OBJ mtllib `../bad.mtl` on line 1 must be a relative source path without labels or `..` segments"
                )
    ));

    let hash_config = database_config("builtin_model_obj_invalid_mtllib_label_path");
    let model_path = AssetPath::parse("models/bad_mtllib_label.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib \"bad.mtl#Variant\"\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl Red\nf 1 2 3\n".to_vec(),
    );
    let mut database = AssetDatabase::new(hash_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_mtllib_label.obj")
                && message.contains(
                    "OBJ mtllib `bad.mtl#Variant` on line 1 must be a relative source path without labels or `..` segments"
                )
    ));
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_accepts_obj_face_outline_aliases() {
    let config = database_config("builtin_model_obj_face_outline_alias");
    let model_path = AssetPath::parse("models/face_outline.obj");
    let mesh_path = AssetPath::parse("models/face_outline.Outline.mesh");
    let model_source = b"o Outline
v 0 0 0
v 1 0 0
v 0 1 0
fo 1 2 3
"
    .to_vec();
    let expected_mesh = b"v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_metadata = database.registry().metadata_by_path(&mesh_path).unwrap();
    let mesh_id = mesh_metadata.id;

    assert_eq!(mesh_metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(mesh_metadata.labels, vec!["Outline"]);
    assert_eq!(mesh_metadata.importer_version, 111);
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![mesh_id]
    );
    assert_eq!(
        fs::read(config.imported_root.join(mesh_path.path())).unwrap(),
        expected_mesh
    );

    let mesh_output = database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "face_outline_alias",
            vec![mesh_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(reader.manifest().dependencies(mesh_id), Some([].as_slice()));
    assert_eq!(reader.read_path(&mesh_path).unwrap(), mesh_output.bytes);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let mesh = server.get_by_id::<Mesh>(mesh_id).unwrap();
    assert_eq!(
        mesh.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
fn database_model_importer_reports_invalid_obj_line_continuation() {
    let config = database_config("builtin_model_obj_dangling_line_continuation");
    let model_path = AssetPath::parse("models/bad_continuation.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        br#"v 0 0 0
f 1 2 \
"#
        .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_continuation.obj")
                && message.contains(
                    "OBJ source line continuation is missing a following line after line 2"
                )
    ));

    let material_config = database_config("builtin_model_obj_dangling_mtl_line_continuation");
    let model_path = AssetPath::parse("models/bad_material_continuation.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        br#"mtllib bad_continuation.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"#
        .to_vec(),
    );
    io.insert(
        "models/bad_continuation.mtl",
        br#"newmtl Red
Kd 0.2 \
"#
        .to_vec(),
    );
    let mut database = AssetDatabase::new(material_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_material_continuation.obj")
                && message.contains(
                    "OBJ material library `bad_continuation.mtl` at `models/bad_continuation.mtl` line continuation is missing a following line after line 2"
                )
    ));
}

#[test]
fn database_model_importer_reports_invalid_obj_uv_and_normal_data() {
    let config = database_config("builtin_model_obj_invalid_tuple_indices");
    let uv_path = AssetPath::parse("models/bad_uv.obj");
    let uv_w_path = AssetPath::parse("models/bad_uv_w.obj");
    let normal_path = AssetPath::parse("models/bad_normal.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        uv_path.path(),
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nf 1/1 2/2 3/1\n".to_vec(),
    );
    io.insert(
        uv_w_path.path(),
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0 1\nf 1/1 2/1 3/1\n".to_vec(),
    );
    io.insert(
        normal_path.path(),
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nvn 0 0 1\nf 1//1 2//2 3//1\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let uv_error = database.import_asset_path(&uv_path).unwrap_err();
    assert!(matches!(
        uv_error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_uv.obj")
                && message.contains(
                    "OBJ texture coordinate index 2 on line 5 references missing texture coordinate"
                )
    ));

    let uv_w_error = database.import_asset_path(&uv_w_path).unwrap_err();
    assert!(matches!(
        uv_w_error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_uv_w.obj")
                && message.contains(
                    "OBJ texture coordinate w component is unsupported because runtime mesh UVs are 2D on line 4"
                )
    ));

    let normal_error = database.import_asset_path(&normal_path).unwrap_err();
    assert!(matches!(
        normal_error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_normal.obj")
                && message.contains("OBJ normal index 2 on line 5 references missing normal")
    ));
}

#[test]
fn database_model_importer_reports_missing_obj_material_texture_dependency() {
    let config = database_config("builtin_model_obj_missing_material_texture");
    let model_path = AssetPath::parse("models/missing_texture.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib missing_texture.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/missing_texture.mtl",
        b"newmtl Red
map_Kd textures/missing.texture
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/missing_texture.obj")
                && message.contains("material dependency `texture.albedo`")
                && message.contains("models/textures/missing.texture")
                && message.contains("is not registered in the asset registry")
    ));
}

#[test]
fn database_model_importer_reports_invalid_obj_material_texture_option() {
    let config = database_config("builtin_model_obj_invalid_material_texture_option");
    let model_path = AssetPath::parse("models/bad_texture_option.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_texture_option.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_texture_option.mtl",
        b"newmtl Red
map_Kd -unknown textures/albedo.texture
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_texture_option.obj")
                && message.contains(
                    "unknown OBJ material library `bad_texture_option.mtl` at `models/bad_texture_option.mtl` map_Kd option -unknown on line 2"
                )
    ));

    for (case, model_name, mtl_name, mtl_source, expected) in [
        (
            "builtin_model_obj_invalid_material_texture_bool_option",
            "bad_texture_bool_option.obj",
            "bad_texture_bool_option.mtl",
            b"newmtl Red
map_Kd -blendu maybe textures/albedo.texture
"
            .as_slice(),
            "OBJ material library `bad_texture_bool_option.mtl` at `models/bad_texture_bool_option.mtl` map_Kd option -blendu value `maybe` on line 2",
        ),
        (
            "builtin_model_obj_invalid_material_texture_bool_option_blendv",
            "bad_texture_bool_option_blendv.obj",
            "bad_texture_bool_option_blendv.mtl",
            b"newmtl Red
map_Kd -blendv maybe textures/albedo.texture
"
            .as_slice(),
            "OBJ material library `bad_texture_bool_option_blendv.mtl` at `models/bad_texture_bool_option_blendv.mtl` map_Kd option -blendv value `maybe` on line 2",
        ),
        (
            "builtin_model_obj_invalid_material_texture_bool_option_cc",
            "bad_texture_bool_option_cc.obj",
            "bad_texture_bool_option_cc.mtl",
            b"newmtl Red
map_Kd -cc maybe textures/albedo.texture
"
            .as_slice(),
            "OBJ material library `bad_texture_bool_option_cc.mtl` at `models/bad_texture_bool_option_cc.mtl` map_Kd option -cc value `maybe` on line 2",
        ),
    ] {
        let bool_config = database_config(case);
        let model_path = AssetPath::parse(&format!("models/{model_name}"));
        let mut io = MemoryAssetIo::new();
        io.insert(
            model_path.path(),
            format!("mtllib {mtl_name}\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl Red\nf 1 2 3\n").into_bytes(),
        );
        io.insert(
            format!("models/{mtl_name}"),
            mtl_source.to_vec(),
        );
        let mut database = AssetDatabase::new(bool_config);
        database.set_io(io);
        database.register_builtin_importers();

        let error = database.import_asset_path(&model_path).unwrap_err();

        assert!(matches!(
            error,
            AssetError::Import { message }
                if message.contains("importer `ModelImporter` failed")
                    && message.contains(&model_path.display_string())
                    && message.contains(expected)
        ));
    }

    let texres_config = database_config("builtin_model_obj_invalid_material_texture_resolution");
    let model_path = AssetPath::parse("models/bad_texture_resolution.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_texture_resolution.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_texture_resolution.mtl",
        b"newmtl Red
map_Kd -texres 0 textures/albedo.texture
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(texres_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_texture_resolution.obj")
                && message.contains(
                    "OBJ material library `bad_texture_resolution.mtl` at `models/bad_texture_resolution.mtl` map_Kd option -texres value `0` on line 2 must be greater than zero"
                )
    ));
}

#[test]
fn database_model_importer_reports_invalid_obj_material_texture_projection() {
    let config = database_config("builtin_model_obj_invalid_material_texture_projection");
    let model_path = AssetPath::parse("models/bad_texture_projection.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_texture_projection.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Reflect
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_texture_projection.mtl",
        b"newmtl Reflect
refl -type cylinder textures/reflection.texture
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_texture_projection.obj")
                && message.contains(
                    "OBJ material library `bad_texture_projection.mtl` at `models/bad_texture_projection.mtl` refl option -type value `cylinder` on line 2"
                )
    ));
}

#[test]
fn database_model_importer_reports_invalid_obj_material_texture_color_space() {
    let config = database_config("builtin_model_obj_invalid_material_texture_colorspace");
    let model_path = AssetPath::parse("models/bad_texture_colorspace.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_texture_colorspace.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_texture_colorspace.mtl",
        b"newmtl Red
map_Kd -colorspace invalid textures/albedo.texture
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_texture_colorspace.obj")
                && message.contains(
                    "OBJ material library `bad_texture_colorspace.mtl` at `models/bad_texture_colorspace.mtl` map_Kd option -colorspace value `invalid` on line 2"
                )
    ));
}

#[test]
fn database_model_importer_reports_invalid_obj_material_texture_boost() {
    let config = database_config("builtin_model_obj_invalid_material_texture_boost");
    let model_path = AssetPath::parse("models/bad_texture_boost.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_texture_boost.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_texture_boost.mtl",
        b"newmtl Red
map_Kd -boost inf textures/albedo.texture
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_texture_boost.obj")
                && message.contains(
                    "OBJ material library `bad_texture_boost.mtl` at `models/bad_texture_boost.mtl` map_Kd option -boost value must be finite on line 2"
                )
    ));
}

#[test]
fn database_model_importer_reports_invalid_obj_material_texture_remap() {
    let config = database_config("builtin_model_obj_invalid_material_texture_remap");
    let model_path = AssetPath::parse("models/bad_texture_remap.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_texture_remap.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_texture_remap.mtl",
        b"newmtl Red
map_Kd -mm 0.2 0.7 nope textures/albedo.texture
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_texture_remap.obj")
                && message.contains(
                    "OBJ material library `bad_texture_remap.mtl` at `models/bad_texture_remap.mtl` map_Kd texture paths on line 2"
                )
    ));
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_material_texture_remap() {
    let config = database_config("builtin_model_obj_material_texture_remap");
    let model_path = AssetPath::parse("models/remap.obj");
    let mesh_path = AssetPath::parse("models/remap.Panel.mesh");
    let material_path = AssetPath::parse("models/remap.Material_Remap.material");
    let albedo_path = AssetPath::parse("models/textures/remap_albedo.texture");
    let model_source = b"mtllib remap.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Remap
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Remap
map_Kd -mm 0.2 0.7 textures/remap_albedo.texture
"
    .to_vec();
    let expected_material = b"# mtllib remap.mtl
name=Remap
texture.albedo=models/textures/remap_albedo.texture
texture.albedo.color_remap=0.2,0.7
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 29);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/remap.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Remap"]);
    assert_eq!(material_metadata.dependencies, vec![albedo_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![albedo_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![albedo_id]
    );

    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_texture_remap",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(material.textures[0].options.color_remap, Some([0.2, 0.7]));
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_material_texture_boost() {
    let config = database_config("builtin_model_obj_material_texture_boost");
    let model_path = AssetPath::parse("models/boost.obj");
    let mesh_path = AssetPath::parse("models/boost.Panel.mesh");
    let material_path = AssetPath::parse("models/boost.Material_Boost.material");
    let albedo_path = AssetPath::parse("models/textures/boost_albedo.texture");
    let model_source = b"mtllib boost.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Boost
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Boost
map_Kd -boost 1.5 textures/boost_albedo.texture
"
    .to_vec();
    let expected_material = b"# mtllib boost.mtl
name=Boost
texture.albedo=models/textures/boost_albedo.texture
texture.albedo.boost=1.5
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 30);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/boost.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Boost"]);
    assert_eq!(material_metadata.dependencies, vec![albedo_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![albedo_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![albedo_id]
    );

    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_texture_boost",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(material.textures[0].options.boost, Some(1.5));
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_material_texture_transform() {
    let config = database_config("builtin_model_obj_material_texture_transform");
    let model_path = AssetPath::parse("models/transform.obj");
    let mesh_path = AssetPath::parse("models/transform.Panel.mesh");
    let material_path = AssetPath::parse("models/transform.Material_Transform.material");
    let albedo_path = AssetPath::parse("models/textures/transform_albedo.texture");
    let model_source = b"mtllib transform.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Transform
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Transform
map_Kd -o 0.25 0.5 0 -s 2 3 1 -t 0.01 0.02 0.03 textures/transform_albedo.texture
"
    .to_vec();
    let expected_material = b"# mtllib transform.mtl
name=Transform
texture.albedo=models/textures/transform_albedo.texture
texture.albedo.transform.offset=0.25,0.5,0
texture.albedo.transform.scale=2,3,1
texture.albedo.transform.turbulence=0.01,0.02,0.03
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 32);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/transform.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Transform"]);
    assert_eq!(material_metadata.dependencies, vec![albedo_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![albedo_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![albedo_id]
    );

    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_texture_transform",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(
        material.textures[0].options.transform.offset,
        [0.25, 0.5, 0.0]
    );
    assert_eq!(
        material.textures[0].options.transform.scale,
        [2.0, 3.0, 1.0]
    );
    assert_eq!(
        material.textures[0].options.transform.turbulence,
        [0.01, 0.02, 0.03]
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_material_texture_resolution() {
    let config = database_config("builtin_model_obj_material_texture_resolution");
    let model_path = AssetPath::parse("models/texres.obj");
    let mesh_path = AssetPath::parse("models/texres.Panel.mesh");
    let material_path = AssetPath::parse("models/texres.Material_Texres.material");
    let albedo_path = AssetPath::parse("models/textures/texres_albedo.texture");
    let model_source = b"mtllib texres.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Texres
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Texres
map_Kd -texres 1024 textures/texres_albedo.texture
"
    .to_vec();
    let expected_material = b"# mtllib texres.mtl
name=Texres
texture.albedo=models/textures/texres_albedo.texture
texture.albedo.texture_resolution=1024
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 31);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/texres.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Texres"]);
    assert_eq!(material_metadata.dependencies, vec![albedo_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![albedo_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![albedo_id]
    );

    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "material_texture_resolution",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(material.textures[0].options.texture_resolution, Some(1024));
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_texture_antialiasing_enabled() {
    let config = database_config("builtin_model_obj_texture_antialiasing_enabled");
    let model_path = AssetPath::parse("models/antialias_on.obj");
    let mesh_path = AssetPath::parse("models/antialias_on.Panel.mesh");
    let material_path = AssetPath::parse("models/antialias_on.Material_Detail.material");
    let model_source = b"mtllib antialias_on.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Detail
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Detail
map_aat ON
"
    .to_vec();
    let expected_material = b"# mtllib antialias_on.mtl
name=Detail
custom.texture_antialias.bool=true
"
    .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/antialias_on.mtl", material_source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Detail"]);
    assert!(material_metadata.dependencies.is_empty());
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    let model_dependencies = &database.registry().get(model_id).unwrap().dependencies;
    assert!(model_dependencies.contains(&mesh_id));
    assert!(model_dependencies.contains(&material_id));

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        Vec::<AssetId>::new()
    );

    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "antialias_on_model",
            vec![mesh_id, material_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(
        material.properties.custom.get("texture_antialias"),
        Some(&MaterialPropertyValue::Bool(true))
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_texture_raw_colorspace() {
    let config = database_config("builtin_model_obj_texture_raw_colorspace");
    let model_path = AssetPath::parse("models/raw_colorspace.obj");
    let mesh_path = AssetPath::parse("models/raw_colorspace.Panel.mesh");
    let material_path = AssetPath::parse("models/raw_colorspace.Material_Raw.material");
    let albedo_path = AssetPath::parse("models/textures/raw_albedo.texture");
    let model_source = b"mtllib raw_colorspace.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Raw
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Raw
map_Kd -colorspace raw textures/raw_albedo.texture
"
    .to_vec();
    let expected_material = b"# mtllib raw_colorspace.mtl
name=Raw
texture.albedo=models/textures/raw_albedo.texture
texture.albedo.color_space=raw
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 23);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/raw_colorspace.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Raw"]);
    assert_eq!(material_metadata.dependencies, vec![albedo_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![albedo_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![albedo_id]
    );

    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "texture_raw_colorspace",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(
        material.textures[0].options.color_space,
        Some(MaterialTextureColorSpace::Raw)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_texture_srgb_colorspace() {
    let config = database_config("builtin_model_obj_texture_srgb_colorspace");
    let model_path = AssetPath::parse("models/srgb_colorspace.obj");
    let mesh_path = AssetPath::parse("models/srgb_colorspace.Panel.mesh");
    let material_path = AssetPath::parse("models/srgb_colorspace.Material_Srgb.material");
    let albedo_path = AssetPath::parse("models/textures/srgb_albedo.texture");
    let model_source = b"mtllib srgb_colorspace.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Srgb
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Srgb
map_Kd -colorspace srgb textures/srgb_albedo.texture
"
    .to_vec();
    let expected_material = b"# mtllib srgb_colorspace.mtl
name=Srgb
texture.albedo=models/textures/srgb_albedo.texture
texture.albedo.color_space=srgb
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 24);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/srgb_colorspace.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Srgb"]);
    assert_eq!(material_metadata.dependencies, vec![albedo_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![albedo_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![albedo_id]
    );

    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "texture_srgb_colorspace",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(
        material.textures[0].options.color_space,
        Some(MaterialTextureColorSpace::Srgb)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_texture_linear_colorspace() {
    let config = database_config("builtin_model_obj_texture_linear_colorspace");
    let model_path = AssetPath::parse("models/linear_colorspace.obj");
    let mesh_path = AssetPath::parse("models/linear_colorspace.Panel.mesh");
    let material_path = AssetPath::parse("models/linear_colorspace.Material_Linear.material");
    let albedo_path = AssetPath::parse("models/textures/linear_albedo.texture");
    let model_source = b"mtllib linear_colorspace.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Linear
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Linear
map_Kd -colorspace linear textures/linear_albedo.texture
"
    .to_vec();
    let expected_material = b"# mtllib linear_colorspace.mtl
name=Linear
texture.albedo=models/textures/linear_albedo.texture
texture.albedo.color_space=linear
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 25);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/linear_colorspace.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Linear"]);
    assert_eq!(material_metadata.dependencies, vec![albedo_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![albedo_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![albedo_id]
    );

    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "texture_linear_colorspace",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(
        material.textures[0].options.color_space,
        Some(MaterialTextureColorSpace::Linear)
    );
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_texture_color_correction() {
    let config = database_config("builtin_model_obj_texture_color_correction");
    let model_path = AssetPath::parse("models/color_correction.obj");
    let mesh_path = AssetPath::parse("models/color_correction.Panel.mesh");
    let material_path = AssetPath::parse("models/color_correction.Material_Correct.material");
    let albedo_path = AssetPath::parse("models/textures/color_correction.texture");
    let model_source = b"mtllib color_correction.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Correct
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Correct
map_Kd -cc ON textures/color_correction.texture
"
    .to_vec();
    let expected_material = b"# mtllib color_correction.mtl
name=Correct
texture.albedo=models/textures/color_correction.texture
texture.albedo.color_correction=true
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 26);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/color_correction.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Correct"]);
    assert_eq!(material_metadata.dependencies, vec![albedo_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![albedo_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![albedo_id]
    );

    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "texture_color_correction",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(material.textures[0].options.color_correction, Some(true));
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_texture_color_correction_disabled() {
    let config = database_config("builtin_model_obj_texture_color_correction_disabled");
    let model_path = AssetPath::parse("models/color_correction_off.obj");
    let mesh_path = AssetPath::parse("models/color_correction_off.Panel.mesh");
    let material_path = AssetPath::parse("models/color_correction_off.Material_Correct.material");
    let albedo_path = AssetPath::parse("models/textures/color_correction_off.texture");
    let model_source = b"mtllib color_correction_off.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Correct
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Correct
map_Kd -cc off textures/color_correction_off.texture
"
    .to_vec();
    let expected_material = b"# mtllib color_correction_off.mtl
name=Correct
texture.albedo=models/textures/color_correction_off.texture
texture.albedo.color_correction=false
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 27);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/color_correction_off.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Correct"]);
    assert_eq!(material_metadata.dependencies, vec![albedo_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![albedo_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![albedo_id]
    );

    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "texture_color_correction_off",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(material.textures[0].options.color_correction, Some(false));
}

#[test]
#[cfg(feature = "bundle")]
fn database_model_importer_preserves_obj_texture_blend_modes() {
    let config = database_config("builtin_model_obj_texture_blend_modes");
    let model_path = AssetPath::parse("models/blend_modes.obj");
    let mesh_path = AssetPath::parse("models/blend_modes.Panel.mesh");
    let material_path = AssetPath::parse("models/blend_modes.Material_Blend.material");
    let albedo_path = AssetPath::parse("models/textures/blend_modes.texture");
    let model_source = b"mtllib blend_modes.mtl
o Panel
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Blend
f 1 2 3
"
    .to_vec();
    let material_source = b"newmtl Blend
map_Kd -blendu ON -blendv OFF textures/blend_modes.texture
"
    .to_vec();
    let expected_material = b"# mtllib blend_modes.mtl
name=Blend
texture.albedo=models/textures/blend_modes.texture
texture.albedo.blend_u=true
texture.albedo.blend_v=false
"
    .to_vec();
    let albedo_source = texture_bytes(1, 1, 28);
    let mut io = MemoryAssetIo::new();
    io.insert(model_path.path(), model_source);
    io.insert("models/blend_modes.mtl", material_source);
    io.insert(albedo_path.path(), albedo_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let albedo_id = database.import_asset_path(&albedo_path).unwrap();
    let model_id = database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_metadata = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap();
    let material_id = material_metadata.id;

    assert_eq!(material_metadata.labels, vec!["Material/Blend"]);
    assert_eq!(material_metadata.dependencies, vec![albedo_id]);
    assert_eq!(
        fs::read(config.imported_root.join(material_path.path())).unwrap(),
        expected_material
    );
    assert_eq!(
        database.registry().get(model_id).unwrap().dependencies,
        vec![albedo_id, mesh_id, material_id]
    );

    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .metadata_by_path(&material_path)
            .unwrap()
            .dependencies,
        vec![albedo_id]
    );

    database
        .cook_asset(albedo_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(mesh_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "texture_blend_modes",
            vec![mesh_id, material_id, albedo_id],
        ))
        .unwrap();
    let reader = BundleReader::from_bytes(&bundle.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(mesh_id),
        Some([material_id].as_slice())
    );
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([albedo_id].as_slice())
    );
    assert_eq!(reader.read_path(&material_path).unwrap(), expected_material);
    assert_eq!(reader.read_path(&albedo_path).unwrap(), albedo_source);

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let material = server.get_by_id::<Material>(material_id).unwrap();
    assert_eq!(material.textures.len(), 1);
    assert_eq!(material.textures[0].name, "albedo");
    assert_eq!(material.textures[0].texture.id(), albedo_id);
    assert_eq!(material.textures[0].options.blend_u, Some(true));
    assert_eq!(material.textures[0].options.blend_v, Some(false));
}

#[test]
fn database_model_importer_reports_unterminated_obj_material_texture_quote() {
    let config = database_config("builtin_model_obj_unterminated_material_texture_quote");
    let model_path = AssetPath::parse("models/bad_texture_quote.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_texture_quote.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_texture_quote.mtl",
        b"newmtl Red
map_Kd \"textures/albedo.texture
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_texture_quote.obj")
                && message.contains(
                    "OBJ material library `bad_texture_quote.mtl` at `models/bad_texture_quote.mtl` has unterminated \" quote on line 2"
                )
    ));
}

#[test]
fn database_model_importer_reports_invalid_obj_material_library_property() {
    let config = database_config("builtin_model_obj_invalid_material_library");
    let model_path = AssetPath::parse("models/bad_material.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad.mtl",
        b"newmtl Red
Kd nope 0 0
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_material.obj")
                && message.contains("invalid OBJ material library `bad.mtl` at `models/bad.mtl` Kd value on line 2")
    ));

    let roughness_config = database_config("builtin_model_obj_invalid_roughness_property");
    let model_path = AssetPath::parse("models/bad_roughness.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_roughness.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_roughness.mtl",
        b"newmtl Red
Pr nope
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(roughness_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_roughness.obj")
                && message.contains("invalid OBJ material library `bad_roughness.mtl` at `models/bad_roughness.mtl` Pr value on line 2")
    ));

    let roughness_range_config = database_config("builtin_model_obj_out_of_range_roughness");
    let model_path = AssetPath::parse("models/bad_roughness_range.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_roughness_range.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_roughness_range.mtl",
        b"newmtl Red
Pr 1.2
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(roughness_range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_roughness_range.obj")
                && message.contains("OBJ material library `bad_roughness_range.mtl` at `models/bad_roughness_range.mtl` Pr value `1.2` on line 2 must be between 0 and 1")
    ));

    let shininess_range_config = database_config("builtin_model_obj_out_of_range_shininess");
    let model_path = AssetPath::parse("models/bad_shininess_range.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_shininess_range.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_shininess_range.mtl",
        b"newmtl Red
Ns 1001
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(shininess_range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_shininess_range.obj")
                && message.contains("OBJ material library `bad_shininess_range.mtl` at `models/bad_shininess_range.mtl` Ns value `1001` on line 2 must be between 0 and 1000")
    ));

    let dissolve_option_config = database_config("builtin_model_obj_invalid_dissolve_option");
    let model_path = AssetPath::parse("models/bad_dissolve.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_dissolve.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_dissolve.mtl",
        b"newmtl Red
d -unknown 0.5
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(dissolve_option_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_dissolve.obj")
                && message.contains("unknown OBJ material library `bad_dissolve.mtl` at `models/bad_dissolve.mtl` d option -unknown on line 2")
    ));

    let pbr_config = database_config("builtin_model_obj_invalid_pbr_property");
    let model_path = AssetPath::parse("models/bad_pbr.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_pbr.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_pbr.mtl",
        b"newmtl Red
Pcr nope
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(pbr_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_pbr.obj")
                && message.contains("invalid OBJ material library `bad_pbr.mtl` at `models/bad_pbr.mtl` Pcr value on line 2")
    ));

    let sheen_range_config = database_config("builtin_model_obj_out_of_range_sheen");
    let model_path = AssetPath::parse("models/bad_sheen_range.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_sheen_range.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_sheen_range.mtl",
        b"newmtl Red
Ps 1.1
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(sheen_range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_sheen_range.obj")
                && message.contains("OBJ material library `bad_sheen_range.mtl` at `models/bad_sheen_range.mtl` Ps value `1.1` on line 2 must be between 0 and 1")
    ));

    let clearcoat_range_config = database_config("builtin_model_obj_out_of_range_clearcoat");
    let model_path = AssetPath::parse("models/bad_clearcoat_range.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_clearcoat_range.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_clearcoat_range.mtl",
        b"newmtl Red
Pc -0.1
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(clearcoat_range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_clearcoat_range.obj")
                && message.contains("OBJ material library `bad_clearcoat_range.mtl` at `models/bad_clearcoat_range.mtl` Pc value `-0.1` on line 2 must be between 0 and 1")
    ));

    let clearcoat_roughness_range_config =
        database_config("builtin_model_obj_out_of_range_clearcoat_roughness");
    let model_path = AssetPath::parse("models/bad_clearcoat_roughness_range.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_clearcoat_roughness_range.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_clearcoat_roughness_range.mtl",
        b"newmtl Red
Pcr 1.1
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(clearcoat_roughness_range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_clearcoat_roughness_range.obj")
                && message.contains("OBJ material library `bad_clearcoat_roughness_range.mtl` at `models/bad_clearcoat_roughness_range.mtl` Pcr value `1.1` on line 2 must be between 0 and 1")
    ));

    let anisotropy_range_config = database_config("builtin_model_obj_out_of_range_anisotropy");
    let model_path = AssetPath::parse("models/bad_anisotropy_range.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_anisotropy_range.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_anisotropy_range.mtl",
        b"newmtl Red
aniso -0.1
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(anisotropy_range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_anisotropy_range.obj")
                && message.contains("OBJ material library `bad_anisotropy_range.mtl` at `models/bad_anisotropy_range.mtl` aniso value `-0.1` on line 2 must be between 0 and 1")
    ));

    let metallic_range_config = database_config("builtin_model_obj_out_of_range_metallic");
    let model_path = AssetPath::parse("models/bad_metallic_range.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_metallic_range.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_metallic_range.mtl",
        b"newmtl Red
Pm -0.1
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(metallic_range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_metallic_range.obj")
                && message.contains("OBJ material library `bad_metallic_range.mtl` at `models/bad_metallic_range.mtl` Pm value `-0.1` on line 2 must be between 0 and 1")
    ));

    let sharpness_config = database_config("builtin_model_obj_invalid_sharpness_property");
    let model_path = AssetPath::parse("models/bad_sharpness.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_sharpness.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_sharpness.mtl",
        b"newmtl Red
sharpness nope
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(sharpness_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_sharpness.obj")
                && message.contains("invalid OBJ material library `bad_sharpness.mtl` at `models/bad_sharpness.mtl` sharpness value on line 2")
    ));

    let sharpness_range_config =
        database_config("builtin_model_obj_out_of_range_sharpness_property");
    let model_path = AssetPath::parse("models/bad_sharpness_range.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_sharpness_range.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_sharpness_range.mtl",
        b"newmtl Red
sharpness 1001
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(sharpness_range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_sharpness_range.obj")
                && message.contains("OBJ material library `bad_sharpness_range.mtl` at `models/bad_sharpness_range.mtl` sharpness value `1001` on line 2 must be between 0 and 1000")
    ));

    let map_aat_config = database_config("builtin_model_obj_invalid_map_aat_property");
    let model_path = AssetPath::parse("models/bad_map_aat.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_map_aat.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_map_aat.mtl",
        b"newmtl Red
map_aat maybe
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(map_aat_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_map_aat.obj")
                && message.contains("invalid OBJ material library `bad_map_aat.mtl` at `models/bad_map_aat.mtl` map_aat value `maybe` on line 2")
    ));

    let illum_config = database_config("builtin_model_obj_invalid_illum_property");
    let model_path = AssetPath::parse("models/bad_illum.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_illum.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_illum.mtl",
        b"newmtl Red
illum nope
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(illum_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_illum.obj")
                && message.contains("invalid OBJ material library `bad_illum.mtl` at `models/bad_illum.mtl` illum value on line 2")
    ));

    let illum_range_config = database_config("builtin_model_obj_out_of_range_illum_property");
    let model_path = AssetPath::parse("models/bad_illum_range.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_illum_range.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_illum_range.mtl",
        b"newmtl Red
illum 11
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(illum_range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_illum_range.obj")
                && message.contains("OBJ material library `bad_illum_range.mtl` at `models/bad_illum_range.mtl` illum value `11` on line 2 must be between 0 and 10")
    ));

    let ni_range_config = database_config("builtin_model_obj_out_of_range_ni_property");
    let model_path = AssetPath::parse("models/bad_ni_range.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_ni_range.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_ni_range.mtl",
        b"newmtl Red
Ni 0
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(ni_range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_ni_range.obj")
                && message.contains("OBJ material library `bad_ni_range.mtl` at `models/bad_ni_range.mtl` Ni value `0` on line 2 must be greater than 0")
    ));

    let dissolve_range_config = database_config("builtin_model_obj_out_of_range_dissolve_property");
    let model_path = AssetPath::parse("models/bad_dissolve_range.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_dissolve_range.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_dissolve_range.mtl",
        b"newmtl Red
d -halo 1.25
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(dissolve_range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_dissolve_range.obj")
                && message.contains("OBJ material library `bad_dissolve_range.mtl` at `models/bad_dissolve_range.mtl` d value `1.25` on line 2 must be between 0 and 1")
    ));

    let transparency_range_config =
        database_config("builtin_model_obj_out_of_range_transparency_property");
    let model_path = AssetPath::parse("models/bad_transparency_range.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_transparency_range.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_transparency_range.mtl",
        b"newmtl Red
Tr -0.1
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(transparency_range_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_transparency_range.obj")
                && message.contains("OBJ material library `bad_transparency_range.mtl` at `models/bad_transparency_range.mtl` Tr value `-0.1` on line 2 must be between 0 and 1")
    ));

    let spectral_config = database_config("builtin_model_obj_unsupported_spectral_color");
    let model_path = AssetPath::parse("models/bad_spectral.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad_spectral.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/bad_spectral.mtl",
        b"newmtl Red
Kd spectral red.rfl 1.0
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(spectral_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/bad_spectral.obj")
                && message.contains("unsupported OBJ material library `bad_spectral.mtl` at `models/bad_spectral.mtl` Kd spectral color on line 2")
    ));
}

#[test]
fn database_model_importer_reports_duplicate_obj_material_names() {
    let same_library_config = database_config("builtin_model_obj_duplicate_material_same_library");
    let model_path = AssetPath::parse("models/duplicate_material.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib duplicate.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert(
        "models/duplicate.mtl",
        b"newmtl Red
Kd 1 0 0
newmtl Red
Kd 0 1 0
"
        .to_vec(),
    );
    let mut database = AssetDatabase::new(same_library_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/duplicate_material.obj")
                && message.contains(
                    "OBJ material library `duplicate.mtl` at `models/duplicate.mtl` defines duplicate newmtl `Red` on line 3"
                )
    ));

    let cross_library_config =
        database_config("builtin_model_obj_duplicate_material_cross_library");
    let model_path = AssetPath::parse("models/duplicate_material_cross.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib red_a.mtl red_b.mtl
v 0 0 0
v 1 0 0
v 0 1 0
usemtl Red
f 1 2 3
"
        .to_vec(),
    );
    io.insert("models/red_a.mtl", b"newmtl Red\nKd 1 0 0\n".to_vec());
    io.insert("models/red_b.mtl", b"newmtl Red\nKd 0 1 0\n".to_vec());
    let mut database = AssetDatabase::new(cross_library_config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/duplicate_material_cross.obj")
                && message.contains(
                    "OBJ material `Red` is defined by multiple mtllib sources: `red_a.mtl` at `models/red_a.mtl` line 1 and `red_b.mtl` at `models/red_b.mtl` line 1"
                )
    ));
}

#[test]
fn database_model_importer_reports_non_utf8_obj_material_library() {
    let config = database_config("builtin_model_obj_non_utf8_material_library");
    let model_path = AssetPath::parse("models/non_utf8_material.obj");
    let mut io = MemoryAssetIo::new();
    io.insert(
        model_path.path(),
        b"mtllib bad.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl Red\nf 1 2 3\n".to_vec(),
    );
    io.insert("models/bad.mtl", vec![0xff, 0xfe, 0xfd]);
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&model_path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ModelImporter` failed")
                && message.contains("models/non_utf8_material.obj")
                && message.contains(
                    "OBJ material library `bad.mtl` at `models/bad.mtl` must be UTF-8"
                )
    ));
}

#[test]
fn database_builtin_material_import_cook_and_runtime_load_preserves_dependencies() {
    let config = database_config("builtin_material_runtime_load");
    let shader_path = AssetPath::parse("shaders/pbr.wgsl");
    let texture_path = AssetPath::parse("textures/albedo.texture");
    let material_path = AssetPath::parse("materials/hero.material");
    let shader_source = b"@fragment fn main() {}\n".to_vec();
    let texture_source = texture_bytes(1, 1, 99);
    let material_source = b"name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo.color_space=linear\ntexture.albedo=textures/albedo.texture\nbase_color=1,0.5,0.25,1\nmetallic=0.2\nroughness=0.7\ndouble_sided=true\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(shader_path.path(), shader_source);
    io.insert(texture_path.path(), texture_source);
    io.insert(material_path.path(), material_source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let shader_id = database.import_asset_path(&shader_path).unwrap();
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    let material_id = database.import_asset_path(&material_path).unwrap();
    assert_eq!(
        database.registry().get(material_id).unwrap().dependencies,
        vec![shader_id, texture_id]
    );

    database
        .cook_asset(shader_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(texture_id, TargetPlatform::Windows)
        .unwrap();
    let output = database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    assert_eq!(output.bytes, material_source);
    assert_eq!(
        fs::read(config.cooked_root.join(material_path.path())).unwrap(),
        material_source
    );
    let metadata = database.registry().get(material_id).unwrap();
    assert_eq!(metadata.dependencies, vec![shader_id, texture_id]);
    assert_eq!(metadata.cooked_path.as_ref(), Some(&material_path));
    assert!(metadata.cooked_hash.is_some());

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.set_registry(database.registry().clone());
    server.register_builtin_loaders();
    let material: Handle<Material> = server.load(material_path);
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.state(&material) == AssetLoadState::Ready {
            break;
        }
    }

    assert!(server.is_ready_with_dependencies(&material));
    assert_eq!(
        server.dependency_graph().direct_dependencies(material.id()),
        &[shader_id, texture_id]
    );
    let loaded = server.get(&material).unwrap();
    assert_eq!(loaded.name.as_deref(), Some("hero"));
    assert_eq!(loaded.shader.as_ref().unwrap().id(), shader_id);
    assert_eq!(loaded.textures[0].texture.id(), texture_id);
    assert_eq!(
        loaded.textures[0].options.color_space,
        Some(MaterialTextureColorSpace::Linear)
    );
    assert_eq!(loaded.properties.base_color, [1.0, 0.5, 0.25, 1.0]);
    assert_eq!(loaded.properties.metallic, 0.2);
    assert_eq!(loaded.properties.roughness, 0.7);
    assert!(loaded.render_state.double_sided);
    assert_eq!(loaded.gpu, Some(GpuResourceHandle(1)));
}

#[test]
fn importer_settings_persist_through_registry_sidecar_and_incremental_scan() {
    let config = database_config("importer_settings_persistence");
    let path = AssetPath::parse("textures/settings.texture");
    let bytes = texture_bytes(1, 1, 18);
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_importer(TextureImporter::new());
    let mut settings = ImporterSettings::default();
    settings.set("quality", "high");
    settings.set("mipmaps", "true");

    let id = database
        .import_asset_path_with_settings(&path, &settings)
        .unwrap();
    let expected_settings = vec![
        ("mipmaps".to_owned(), "true".to_owned()),
        ("quality".to_owned(), "high".to_owned()),
    ];
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.importer_settings, expected_settings);
    assert!(metadata.settings_hash.is_some());
    database.save_registry().unwrap();
    database.save_all_metadata_sidecars().unwrap();

    let mut loaded_registry = AssetDatabase::new(config.clone());
    loaded_registry.load_registry().unwrap();
    assert_eq!(
        loaded_registry
            .registry()
            .get(id)
            .unwrap()
            .importer_settings,
        expected_settings
    );

    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .get(id)
            .unwrap()
            .importer_settings,
        expected_settings
    );

    let mut scan_io = MemoryAssetIo::new();
    scan_io.insert(path.path(), bytes);
    loaded_sidecars.set_io(scan_io);
    let report = loaded_sidecars.scan_with_metadata().unwrap();
    assert_eq!(report.unchanged, vec![path.clone()]);
    assert_eq!(
        loaded_sidecars
            .registry()
            .get(id)
            .unwrap()
            .importer_settings,
        expected_settings
    );
}

#[test]
fn database_builtin_texture_cooker_writes_runtime_loadable_output() {
    let config = database_config("builtin_cooker_runtime_load");
    let path = AssetPath::parse("textures/cooked.texture");
    let bytes = texture_bytes(2, 1, 123);
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

    assert_eq!(output.bytes, bytes);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        bytes
    );
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert!(metadata.cooked_hash.is_some());

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let texture: Handle<Texture> = server.load(path);
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(1))),
    );

    assert!(server.is_ready(&texture));
    assert_eq!(
        (
            server.get(&texture).unwrap().width,
            server.get(&texture).unwrap().height
        ),
        (2, 1)
    );
}

#[test]
fn database_texture_cooker_canonicalizes_runtime_and_source_bytes() {
    let runtime_path = AssetPath::parse("textures/cooked.texture");
    let runtime_bytes = texture_bytes(2, 1, 17);
    let runtime_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(runtime_path.clone()),
        source_bytes: runtime_bytes.clone(),
    };
    let runtime_metadata = AssetMetadata::runtime(
        AssetId::new(),
        runtime_path,
        AssetTypeId::of::<Texture>(),
    );
    let source_path = AssetPath::parse("textures/from_source.texture");
    let source_bytes = b"NGA_TEXTURE_SOURCE_V1\nsize=2x1\nrgba=17,17,17,17,17,17,17,17\n".to_vec();
    let source_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(source_path.clone()),
        source_bytes: source_bytes.clone(),
    };
    let source_metadata = AssetMetadata::runtime(
        AssetId::new(),
        source_path,
        AssetTypeId::of::<Texture>(),
    );
    let cooker = TextureCooker::new();

    let runtime_output = cooker.cook(&runtime_ctx, &runtime_metadata).unwrap();
    let source_output = cooker.cook(&source_ctx, &source_metadata).unwrap();

    assert_eq!(runtime_output.bytes, runtime_bytes);
    assert_eq!(runtime_output.version_hash, VersionHash(2));
    assert_eq!(runtime_output.metadata, runtime_metadata);
    assert_eq!(source_output.bytes, runtime_bytes);
    assert_eq!(source_output.version_hash, VersionHash(2));
    assert_eq!(source_output.metadata, source_metadata);
}

#[test]
fn database_audio_cooker_canonicalizes_runtime_and_wav_bytes() {
    let runtime_path = AssetPath::parse("audio/cooked.audio");
    let runtime_bytes = audio_bytes();
    let runtime_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(runtime_path.clone()),
        source_bytes: runtime_bytes.clone(),
    };
    let runtime_metadata = AssetMetadata::runtime(
        AssetId::new(),
        runtime_path,
        AssetTypeId::of::<AudioClip>(),
    );
    let wav_path = AssetPath::parse("audio/from_wav.audio");
    let mut wav_samples = Vec::new();
    for sample in [0.0f32, 0.5, -0.5] {
        wav_samples.extend_from_slice(&sample.to_le_bytes());
    }
    let wav_bytes = wav_format_bytes(44_100, 1, 3, 32, &wav_samples);
    let wav_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(wav_path.clone()),
        source_bytes: wav_bytes.clone(),
    };
    let wav_metadata = AssetMetadata::runtime(
        AssetId::new(),
        wav_path,
        AssetTypeId::of::<AudioClip>(),
    );
    let cooker = AudioCooker::new();
    let expected = b"NGA_AUDIO_V1\nsample_rate=44100\nchannels=1\nformat=f32\nsamples=0,0.5,-0.5\nstreaming=false\n".to_vec();

    let runtime_output = cooker.cook(&runtime_ctx, &runtime_metadata).unwrap();
    let wav_output = cooker.cook(&wav_ctx, &wav_metadata).unwrap();

    assert_eq!(runtime_output.bytes, expected);
    assert_eq!(runtime_output.version_hash, VersionHash(8));
    assert_eq!(runtime_output.metadata, runtime_metadata);
    assert_eq!(wav_output.bytes, expected);
    assert_eq!(wav_output.version_hash, VersionHash(8));
    assert_eq!(wav_output.metadata, wav_metadata);
}

#[test]
fn database_builtin_scene_and_prefab_importers_and_cookers_preserve_dependencies_and_load_runtime_docs(
) {
    let config = database_config("builtin_scene_prefab_import_cook_load");
    let shader_path = AssetPath::parse("shaders/pbr.wgsl");
    let texture_path = AssetPath::parse("textures/hero.texture");
    let mesh_path = AssetPath::parse("meshes/tri.mesh");
    let material_path = AssetPath::parse("materials/hero.material");
    let scene_path = AssetPath::parse("scenes/hero.scene");
    let prefab_path = AssetPath::parse("prefabs/hero.prefab");
    let mut io = MemoryAssetIo::new();
    io.insert(shader_path.path(), shader_bytes());
    io.insert(texture_path.path(), texture_bytes(1, 1, 17));
    io.insert(mesh_path.path(), mesh_bytes());
    io.insert(
        material_path.path(),
        b"name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/hero.texture\n".to_vec(),
    );
    io.insert(
        scene_path.path(),
        b"NGA_SCENE_V1\nname=hero_scene\ndependency=textures/hero.texture\ndependency=materials/hero.material\nentity=Root\ncomponent=Transform|translation=0,0,0\nentity=Hero;parent=0\ncomponent=MeshRenderer|mesh=meshes/tri.mesh;material=materials/hero.material\n".to_vec(),
    );
    io.insert(
        prefab_path.path(),
        b"NGA_PREFAB_V1\ndependency=textures/hero.texture\ndependency=materials/hero.material\nroot=Hero\ncomponent=Transform|translation=0,0,0\nchild=Weapon;parent=0\ncomponent=MeshRenderer|mesh=meshes/tri.mesh;material=materials/hero.material\n".to_vec(),
    );

    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let shader_id = database.import_asset_path(&shader_path).unwrap();
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    let mesh_id = database.import_asset_path(&mesh_path).unwrap();
    let material_id = database.import_asset_path(&material_path).unwrap();
    let scene_id = database.import_asset_path(&scene_path).unwrap();
    let prefab_id = database.import_asset_path(&prefab_path).unwrap();

    assert_eq!(
        database.registry().get(scene_id).unwrap().dependencies,
        vec![texture_id, material_id, mesh_id]
    );
    assert_eq!(
        database.registry().get(prefab_id).unwrap().dependencies,
        vec![texture_id, material_id, mesh_id]
    );
    assert_eq!(
        database.registry().get(material_id).unwrap().dependencies,
        vec![shader_id, texture_id]
    );

    for id in [
        shader_id,
        texture_id,
        mesh_id,
        material_id,
        scene_id,
        prefab_id,
    ] {
        database.cook_asset(id, TargetPlatform::Windows).unwrap();
    }

    assert_eq!(
        fs::read(config.cooked_root.join(scene_path.path())).unwrap(),
        fs::read(config.imported_root.join(scene_path.path())).unwrap()
    );
    assert_eq!(
        fs::read(config.cooked_root.join(prefab_path.path())).unwrap(),
        fs::read(config.imported_root.join(prefab_path.path())).unwrap()
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let shader: Handle<Shader> = server.load(shader_path);
    let texture: Handle<Texture> = server.load(texture_path);
    let mesh: Handle<Mesh> = server.load(mesh_path);
    let material: Handle<Material> = server.load(material_path);
    let scene: Handle<SceneAsset> = server.load(scene_path);
    let prefab: Handle<Prefab> = server.load(prefab_path);

    for _ in 0..8 {
        server.update_loading();
        let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
        server.finish_gpu_uploads(uploads.into_iter().enumerate().map(|(index, upload)| {
            GpuUploadResult::ok(upload.id, GpuResourceHandle(index as u64 + 1))
        }));
        if server.is_ready_with_dependencies(&shader)
            && server.is_ready_with_dependencies(&texture)
            && server.is_ready_with_dependencies(&mesh)
            && server.is_ready_with_dependencies(&material)
            && server.is_ready_with_dependencies(&scene)
            && server.is_ready_with_dependencies(&prefab)
        {
            break;
        }
    }

    assert!(server.is_ready_with_dependencies(&shader));
    assert!(server.is_ready_with_dependencies(&texture));
    assert!(server.is_ready_with_dependencies(&mesh));
    assert!(server.is_ready_with_dependencies(&material));
    assert!(server.is_ready_with_dependencies(&scene));
    assert!(server.is_ready_with_dependencies(&prefab));
    assert_eq!(
        server
            .dependency_graph()
            .direct_dependencies(scene.id())
            .len(),
        3
    );
    assert_eq!(
        server
            .dependency_graph()
            .direct_dependencies(prefab.id())
            .len(),
        3
    );
}

#[test]
fn database_scene_and_prefab_importers_preserve_runtime_documents_and_dependencies() {
    let scene_path = AssetPath::parse("scenes/imported.scene");
    let prefab_path = AssetPath::parse("prefabs/imported.prefab");
    let texture_path = AssetPath::parse("textures/shared.texture");
    let material_path = AssetPath::parse("materials/shared.material");
    let mesh_path = AssetPath::parse("meshes/shared.mesh");
    let scene_bytes =
        b"NGA_SCENE_V1\nname=imported_scene\ndependency=textures/shared.texture\ndependency=materials/shared.material\nentity=Root\ncomponent=Transform|translation=0,0,0\nentity=Hero;parent=0\ncomponent=MeshRenderer|mesh=meshes/shared.mesh;material=materials/shared.material\n".to_vec();
    let prefab_bytes =
        b"NGA_PREFAB_V1\ndependency=textures/shared.texture\ndependency=materials/shared.material\nroot=Root\ncomponent=Transform|translation=0,0,0\nchild=Hero;parent=0\ncomponent=MeshRenderer|mesh=meshes/shared.mesh;material=materials/shared.material\n".to_vec();
    let texture_id = AssetId::new();
    let material_id = AssetId::new();
    let mesh_id = AssetId::new();
    let mut registry = AssetRegistry::new();
    registry.insert(AssetMetadata::runtime(
        texture_id,
        texture_path.clone(),
        AssetTypeId::of::<Texture>(),
    ));
    registry.insert(AssetMetadata::runtime(
        material_id,
        material_path.clone(),
        AssetTypeId::of::<Material>(),
    ));
    registry.insert(AssetMetadata::runtime(
        mesh_id,
        mesh_path.clone(),
        AssetTypeId::of::<Mesh>(),
    ));
    let scene_source = SourceAsset {
        path: scene_path.clone(),
        bytes: scene_bytes.clone(),
        hash: ContentHash(0),
    };
    let prefab_source = SourceAsset {
        path: prefab_path.clone(),
        bytes: prefab_bytes.clone(),
        hash: ContentHash(0),
    };
    let scene_importer = SceneImporter::new();
    let prefab_importer = PrefabImporter::new();
    let settings = ImporterSettings::default();
    let mut scene_ctx = ImportContext::with_registry(&registry);
    let mut prefab_ctx = ImportContext::with_registry(&registry);

    let scene_output = scene_importer
        .import(&mut scene_ctx, &scene_source, &settings)
        .unwrap();
    let prefab_output = prefab_importer
        .import(&mut prefab_ctx, &prefab_source, &settings)
        .unwrap();

    assert_eq!(scene_output.metadata.path.as_ref(), Some(&scene_path));
    assert_eq!(scene_output.metadata.importer.as_deref(), Some("SceneImporter"));
    assert_eq!(scene_output.metadata.importer_version, 1);
    assert_eq!(scene_output.metadata.source_path.as_ref(), Some(&scene_path));
    assert_eq!(scene_output.metadata.cooked_path.as_ref(), Some(&scene_path));
    assert_eq!(scene_output.metadata.version_hash, Some(VersionHash(1)));
    assert_eq!(scene_output.version_hash, VersionHash(1));
    assert_eq!(scene_output.generated.len(), 1);
    assert_eq!(scene_output.generated[0].bytes, scene_bytes);
    assert_eq!(scene_output.generated[0].path, scene_path);
    assert_eq!(scene_output.dependencies, vec![texture_id, material_id, mesh_id]);

    assert_eq!(prefab_output.metadata.path.as_ref(), Some(&prefab_path));
    assert_eq!(prefab_output.metadata.importer.as_deref(), Some("PrefabImporter"));
    assert_eq!(prefab_output.metadata.importer_version, 1);
    assert_eq!(prefab_output.metadata.source_path.as_ref(), Some(&prefab_path));
    assert_eq!(prefab_output.metadata.cooked_path.as_ref(), Some(&prefab_path));
    assert_eq!(prefab_output.metadata.version_hash, Some(VersionHash(1)));
    assert_eq!(prefab_output.version_hash, VersionHash(1));
    assert_eq!(prefab_output.generated.len(), 1);
    assert_eq!(prefab_output.generated[0].bytes, prefab_bytes);
    assert_eq!(prefab_output.generated[0].path, prefab_path);
    assert_eq!(prefab_output.dependencies, vec![texture_id, material_id, mesh_id]);
}

#[test]
fn database_texture_and_shader_importers_preserve_runtime_documents_and_metadata() {
    let texture_path = AssetPath::parse("textures/imported.texture");
    let shader_path = AssetPath::parse("shaders/imported.wgsl");
    let texture_bytes = texture_bytes(1, 1, 17);
    let shader_runtime_bytes = b"@fragment fn main() {}\n".to_vec();
    let texture_source = SourceAsset {
        path: texture_path.clone(),
        bytes: b"NGA_TEXTURE_SOURCE_V1\nsize=1x1\nrgba=17,17,17,17\n".to_vec(),
        hash: ContentHash(101),
    };
    let shader_source = SourceAsset {
        path: shader_path.clone(),
        bytes: b"NGA_SHADER_SOURCE_V1\nlanguage=wgsl\nstage=fragment\nentry=main\n---\n@fragment fn main() {}\n"
            .to_vec(),
        hash: ContentHash(202),
    };
    let texture_importer = TextureImporter::new();
    let shader_importer = ShaderImporter::new();
    let settings = ImporterSettings::default();
    let mut texture_ctx = ImportContext::default();
    let mut shader_ctx = ImportContext::default();

    let texture_output = texture_importer
        .import(&mut texture_ctx, &texture_source, &settings)
        .unwrap();
    let shader_output = shader_importer
        .import(&mut shader_ctx, &shader_source, &settings)
        .unwrap();

    assert_eq!(texture_output.metadata.path.as_ref(), Some(&texture_path));
    assert_eq!(
        texture_output.metadata.importer.as_deref(),
        Some("TextureImporter")
    );
    assert_eq!(texture_output.metadata.importer_version, 3);
    assert_eq!(texture_output.metadata.source_path.as_ref(), Some(&texture_path));
    assert_eq!(texture_output.metadata.cooked_path.as_ref(), Some(&texture_path));
    assert_eq!(texture_output.metadata.source_hash, Some(ContentHash(101)));
    assert_eq!(texture_output.metadata.version_hash, Some(VersionHash(3)));
    assert_eq!(texture_output.version_hash, VersionHash(3));
    assert_eq!(texture_output.generated.len(), 1);
    assert_eq!(texture_output.generated[0].path, texture_path);
    assert_eq!(texture_output.generated[0].bytes, texture_bytes);
    assert_eq!(texture_output.generated[0].asset_type, Texture::TYPE_ID);
    assert!(texture_output.dependencies.is_empty());

    assert_eq!(shader_output.metadata.path.as_ref(), Some(&shader_path));
    assert_eq!(
        shader_output.metadata.importer.as_deref(),
        Some("ShaderImporter")
    );
    assert_eq!(shader_output.metadata.importer_version, 3);
    assert_eq!(shader_output.metadata.source_path.as_ref(), Some(&shader_path));
    assert_eq!(shader_output.metadata.cooked_path.as_ref(), Some(&shader_path));
    assert_eq!(shader_output.metadata.source_hash, Some(ContentHash(202)));
    assert_eq!(shader_output.metadata.version_hash, Some(VersionHash(3)));
    assert_eq!(shader_output.version_hash, VersionHash(3));
    assert_eq!(shader_output.generated.len(), 1);
    assert_eq!(shader_output.generated[0].path, shader_path);
    assert_eq!(shader_output.generated[0].bytes, shader_runtime_bytes);
    assert_eq!(shader_output.generated[0].asset_type, Shader::TYPE_ID);
    assert!(shader_output.dependencies.is_empty());
}

#[test]
fn database_audio_font_and_physics_mesh_importers_preserve_runtime_documents_and_metadata() {
    let audio_path = AssetPath::parse("audio/imported.audio");
    let font_path = AssetPath::parse("fonts/imported.font");
    let physics_path = AssetPath::parse("physics/imported.physics");
    let audio_source = SourceAsset {
        path: audio_path.clone(),
        bytes: b"NGA_AUDIO_SOURCE_V1\nsample_rate=44100\nchannels=1\nformat=f32\nsamples=0.0,0.5,-0.5\nstreaming=false\n"
            .to_vec(),
        hash: ContentHash(303),
    };
    let font_source = SourceAsset {
        path: font_path.clone(),
        bytes: b"NGA_FONT_SOURCE_V1\n# canonical order should not depend on source order\nglyph = char=B; size=1x1; bitmap=128\nfamily = Debug Sans\nglyph=char=A;size=2x1;bitmap=0, 255\n"
            .to_vec(),
        hash: ContentHash(404),
    };
    let physics_source = SourceAsset {
        path: physics_path.clone(),
        bytes: b"NGA_PHYSICS_MESH_SOURCE_V1\n# source accepts key/value and directive forms\nkind = tri_mesh\nvertex = 0.0, 0.0, 0.0\nv 1.50 0 0\nv 0 1 0\ntriangle = 0, 1, 2\n"
            .to_vec(),
        hash: ContentHash(505),
    };
    let audio_importer = AudioImporter::new();
    let font_importer = FontImporter::new();
    let physics_importer = PhysicsMeshImporter::new();
    let settings = ImporterSettings::default();
    let mut audio_ctx = ImportContext::default();
    let mut font_ctx = ImportContext::default();
    let mut physics_ctx = ImportContext::default();

    let audio_output = audio_importer
        .import(&mut audio_ctx, &audio_source, &settings)
        .unwrap();
    let font_output = font_importer
        .import(&mut font_ctx, &font_source, &settings)
        .unwrap();
    let physics_output = physics_importer
        .import(&mut physics_ctx, &physics_source, &settings)
        .unwrap();

    assert_eq!(audio_output.metadata.path.as_ref(), Some(&audio_path));
    assert_eq!(audio_output.metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(audio_output.metadata.importer_version, 3);
    assert_eq!(audio_output.metadata.source_path.as_ref(), Some(&audio_path));
    assert_eq!(audio_output.metadata.cooked_path.as_ref(), Some(&audio_path));
    assert_eq!(audio_output.metadata.source_hash, Some(ContentHash(303)));
    assert_eq!(audio_output.metadata.version_hash, Some(VersionHash(3)));
    assert_eq!(audio_output.version_hash, VersionHash(3));
    assert_eq!(audio_output.generated.len(), 1);
    assert_eq!(audio_output.generated[0].path, audio_path);
    assert_eq!(
        audio_output.generated[0].bytes,
        b"NGA_AUDIO_V1\nsample_rate=44100\nchannels=1\nformat=f32\nsamples=0,0.5,-0.5\nstreaming=false\n"
            .to_vec()
    );
    assert_eq!(audio_output.generated[0].asset_type, AudioClip::TYPE_ID);
    assert!(audio_output.dependencies.is_empty());

    assert_eq!(font_output.metadata.path.as_ref(), Some(&font_path));
    assert_eq!(font_output.metadata.importer.as_deref(), Some("FontImporter"));
    assert_eq!(font_output.metadata.importer_version, 3);
    assert_eq!(font_output.metadata.source_path.as_ref(), Some(&font_path));
    assert_eq!(font_output.metadata.cooked_path.as_ref(), Some(&font_path));
    assert_eq!(font_output.metadata.source_hash, Some(ContentHash(404)));
    assert_eq!(font_output.metadata.version_hash, Some(VersionHash(3)));
    assert_eq!(font_output.version_hash, VersionHash(3));
    assert_eq!(font_output.generated.len(), 1);
    assert_eq!(font_output.generated[0].path, font_path);
    assert_eq!(
        font_output.generated[0].bytes,
        b"NGA_FONT_V1\nfamily=Debug Sans\nglyph=char=A;size=2x1;bitmap=0,255\nglyph=char=B;size=1x1;bitmap=128\n"
            .to_vec()
    );
    assert_eq!(font_output.generated[0].asset_type, Font::TYPE_ID);
    assert!(font_output.dependencies.is_empty());

    assert_eq!(physics_output.metadata.path.as_ref(), Some(&physics_path));
    assert_eq!(
        physics_output.metadata.importer.as_deref(),
        Some("PhysicsMeshImporter")
    );
    assert_eq!(physics_output.metadata.importer_version, 2);
    assert_eq!(physics_output.metadata.source_path.as_ref(), Some(&physics_path));
    assert_eq!(physics_output.metadata.cooked_path.as_ref(), Some(&physics_path));
    assert_eq!(physics_output.metadata.source_hash, Some(ContentHash(505)));
    assert_eq!(physics_output.metadata.version_hash, Some(VersionHash(2)));
    assert_eq!(physics_output.version_hash, VersionHash(2));
    assert_eq!(physics_output.generated.len(), 1);
    assert_eq!(physics_output.generated[0].path, physics_path);
    assert_eq!(
        physics_output.generated[0].bytes,
        b"NGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1.5 0 0\nv 0 1 0\ni 0 1 2\n"
            .to_vec()
    );
    assert_eq!(physics_output.generated[0].asset_type, PhysicsMesh::TYPE_ID);
    assert!(physics_output.dependencies.is_empty());
}

#[test]
fn database_builtin_animation_and_skeleton_importers_and_cookers_preserve_runtime_docs() {
    let config = database_config("builtin_animation_skeleton_import_cook_load");
    let animation_path = AssetPath::parse("animations/hero.animation");
    let skeleton_path = AssetPath::parse("skeletons/hero.skeleton");
    let mut io = MemoryAssetIo::new();
    io.insert(animation_path.path(), animation_source_bytes());
    io.insert(skeleton_path.path(), skeleton_source_bytes());

    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let animation_id = database.import_asset_path(&animation_path).unwrap();
    let skeleton_id = database.import_asset_path(&skeleton_path).unwrap();

    assert_eq!(
        database.registry().get(animation_id).unwrap().asset_type,
        AnimationClip::TYPE_ID
    );
    assert_eq!(
        database.registry().get(skeleton_id).unwrap().asset_type,
        Skeleton::TYPE_ID
    );
    assert!(database
        .registry()
        .get(animation_id)
        .unwrap()
        .dependencies
        .is_empty());
    assert!(database
        .registry()
        .get(skeleton_id)
        .unwrap()
        .dependencies
        .is_empty());

    database
        .cook_asset(animation_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(skeleton_id, TargetPlatform::Windows)
        .unwrap();

    assert_eq!(
        fs::read(config.imported_root.join(animation_path.path())).unwrap(),
        animation_runtime_bytes()
    );
    assert_eq!(
        fs::read(config.cooked_root.join(animation_path.path())).unwrap(),
        fs::read(config.imported_root.join(animation_path.path())).unwrap()
    );
    assert_eq!(
        fs::read(config.cooked_root.join(skeleton_path.path())).unwrap(),
        fs::read(config.imported_root.join(skeleton_path.path())).unwrap()
    );

    let mut server_io = MemoryAssetIo::new();
    server_io.insert(
        animation_path.path(),
        fs::read(config.cooked_root.join(animation_path.path())).unwrap(),
    );
    server_io.insert(
        skeleton_path.path(),
        fs::read(config.cooked_root.join(skeleton_path.path())).unwrap(),
    );
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(server_io);
    server.register_builtin_loaders();
    let animation: Handle<AnimationClip> = server.load(animation_path);
    let skeleton: Handle<Skeleton> = server.load(skeleton_path);

    for _ in 0..4 {
        server.update_loading();
        if server.is_ready(&animation) && server.is_ready(&skeleton) {
            break;
        }
    }

    assert!(server.is_ready(&animation));
    assert!(server.is_ready(&skeleton));
}

#[test]
fn database_animation_and_skeleton_cookers_canonicalize_source_documents() {
    let config = database_config("animation_skeleton_cooker_source_conversion");
    let animation_path = AssetPath::parse("animations/source.animation");
    let skeleton_path = AssetPath::parse("skeletons/source.skeleton");
    let invalid_animation_path = AssetPath::parse("animations/invalid.animation");
    let invalid_skeleton_path = AssetPath::parse("skeletons/invalid.skeleton");
    let mut io = MemoryAssetIo::new();
    io.insert(animation_path.path(), animation_source_bytes());
    io.insert(skeleton_path.path(), skeleton_source_bytes());
    io.insert(
        invalid_animation_path.path(),
        b"NGA_ANIMATION_SOURCE_V1\nduration=-1.0\nticks_per_second=24.0\ntrack=node:Hero\ntranslation=0.0:0,0,0\n"
            .to_vec(),
    );
    io.insert(
        invalid_skeleton_path.path(),
        b"NGA_SKELETON_SOURCE_V1\nbone=Root\nbone=Root\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_cookers();

    let animation_id = AssetId::from_u128(0x4e47_4153_5345_5400_0000_0000_0000_a111);
    let skeleton_id = AssetId::from_u128(0x4e47_4153_5345_5400_0000_0000_0000_a112);
    let invalid_animation_id = AssetId::from_u128(0x4e47_4153_5345_5400_0000_0000_0000_a113);
    let invalid_skeleton_id = AssetId::from_u128(0x4e47_4153_5345_5400_0000_0000_0000_a114);
    database.registry_mut().insert(AssetMetadata::runtime(
        animation_id,
        animation_path.clone(),
        AnimationClip::TYPE_ID,
    ));
    database.registry_mut().insert(AssetMetadata::runtime(
        skeleton_id,
        skeleton_path.clone(),
        Skeleton::TYPE_ID,
    ));
    database.registry_mut().insert(AssetMetadata::runtime(
        invalid_animation_id,
        invalid_animation_path,
        AnimationClip::TYPE_ID,
    ));
    database.registry_mut().insert(AssetMetadata::runtime(
        invalid_skeleton_id,
        invalid_skeleton_path,
        Skeleton::TYPE_ID,
    ));

    let animation_output = database
        .cook_asset(animation_id, TargetPlatform::Windows)
        .unwrap();
    let skeleton_output = database
        .cook_asset(skeleton_id, TargetPlatform::Windows)
        .unwrap();

    assert_eq!(animation_output.bytes, animation_runtime_bytes());
    assert_eq!(animation_output.version_hash, VersionHash(2));
    assert_eq!(skeleton_output.bytes, skeleton_runtime_bytes());
    assert_eq!(skeleton_output.version_hash, VersionHash(2));
    assert_eq!(
        fs::read(config.cooked_root.join(animation_path.path())).unwrap(),
        animation_runtime_bytes()
    );
    assert_eq!(
        fs::read(config.cooked_root.join(skeleton_path.path())).unwrap(),
        skeleton_runtime_bytes()
    );
    assert!(matches!(
        database.cook_asset(invalid_animation_id, TargetPlatform::Windows),
        Err(AssetError::Cook { message })
            if message.contains("AnimationCooker")
                && message.contains("failed to validate animation source")
                && message.contains("duration must be greater than zero")
    ));
    assert!(matches!(
        database.cook_asset(invalid_skeleton_id, TargetPlatform::Windows),
        Err(AssetError::Cook { message })
            if message.contains("SkeletonCooker")
                && message.contains("failed to validate skeleton source")
                && message.contains("duplicates an earlier bone name")
    ));

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let animation: Handle<AnimationClip> = server.load(animation_path);
    let skeleton: Handle<Skeleton> = server.load(skeleton_path);
    for _ in 0..4 {
        server.update_loading();
        if server.is_ready(&animation) && server.is_ready(&skeleton) {
            break;
        }
    }

    assert!(server.is_ready(&animation));
    assert!(server.is_ready(&skeleton));
}

#[test]
fn database_animation_and_skeleton_cookers_canonicalize_runtime_and_source_bytes() {
    let animation_runtime_path = AssetPath::parse("animations/cooked_runtime.animation");
    let skeleton_runtime_path = AssetPath::parse("skeletons/cooked_runtime.skeleton");
    let animation_runtime_expected = animation_runtime_bytes();
    let skeleton_runtime_expected = skeleton_runtime_bytes();
    let animation_runtime_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(animation_runtime_path.clone()),
        source_bytes: animation_runtime_expected.clone(),
    };
    let skeleton_runtime_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(skeleton_runtime_path.clone()),
        source_bytes: skeleton_runtime_expected.clone(),
    };
    let animation_runtime_metadata = AssetMetadata::runtime(
        AssetId::new(),
        animation_runtime_path,
        AssetTypeId::of::<AnimationClip>(),
    );
    let skeleton_runtime_metadata = AssetMetadata::runtime(
        AssetId::new(),
        skeleton_runtime_path,
        AssetTypeId::of::<Skeleton>(),
    );
    let animation_source_path = AssetPath::parse("animations/from_source.animation");
    let skeleton_source_path = AssetPath::parse("skeletons/from_source.skeleton");
    let animation_source_expected = animation_runtime_bytes();
    let skeleton_source_expected = skeleton_runtime_bytes();
    let animation_source_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(animation_source_path.clone()),
        source_bytes: animation_source_bytes(),
    };
    let skeleton_source_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(skeleton_source_path.clone()),
        source_bytes: skeleton_source_bytes(),
    };
    let animation_source_metadata = AssetMetadata::runtime(
        AssetId::new(),
        animation_source_path,
        AssetTypeId::of::<AnimationClip>(),
    );
    let skeleton_source_metadata = AssetMetadata::runtime(
        AssetId::new(),
        skeleton_source_path,
        AssetTypeId::of::<Skeleton>(),
    );
    let animation_cooker = AnimationCooker::new();
    let skeleton_cooker = SkeletonCooker::new();

    let animation_runtime_output = animation_cooker
        .cook(&animation_runtime_ctx, &animation_runtime_metadata)
        .unwrap();
    let skeleton_runtime_output = skeleton_cooker
        .cook(&skeleton_runtime_ctx, &skeleton_runtime_metadata)
        .unwrap();
    let animation_source_output = animation_cooker
        .cook(&animation_source_ctx, &animation_source_metadata)
        .unwrap();
    let skeleton_source_output = skeleton_cooker
        .cook(&skeleton_source_ctx, &skeleton_source_metadata)
        .unwrap();

    assert_eq!(animation_runtime_output.bytes, animation_runtime_expected);
    assert_eq!(animation_runtime_output.version_hash, VersionHash(2));
    assert_eq!(animation_runtime_output.metadata, animation_runtime_metadata);
    assert_eq!(skeleton_runtime_output.bytes, skeleton_runtime_expected);
    assert_eq!(skeleton_runtime_output.version_hash, VersionHash(2));
    assert_eq!(skeleton_runtime_output.metadata, skeleton_runtime_metadata);
    assert_eq!(animation_source_output.bytes, animation_source_expected);
    assert_eq!(animation_source_output.version_hash, VersionHash(2));
    assert_eq!(animation_source_output.metadata, animation_source_metadata);
    assert_eq!(skeleton_source_output.bytes, skeleton_source_expected);
    assert_eq!(skeleton_source_output.version_hash, VersionHash(2));
    assert_eq!(skeleton_source_output.metadata, skeleton_source_metadata);
}

#[test]
fn database_texture_cooker_canonicalizes_source_documents() {
    let config = database_config("texture_cooker_source_conversion");
    let path = AssetPath::parse("textures/generated.texture");
    let source = b"NGA_TEXTURE_SOURCE_V1\nsize=2x1\nrgba=255,0,0,255;0,255,0,255\n".to_vec();
    let expected = texture_rgba_bytes(2, 1, &[255, 0, 0, 255, 0, 255, 0, 255]);
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let texture: Handle<Texture> = server.load(path);
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(11))),
    );

    assert!(server.is_ready(&texture));
    assert_eq!(
        (
            server.get(&texture).unwrap().width,
            server.get(&texture).unwrap().height
        ),
        (2, 1)
    );
}

#[test]
fn database_texture_importer_converts_text_source_to_runtime_texture_bytes() {
    let config = database_config("texture_importer_source_conversion");
    let path = AssetPath::parse("textures/generated.texture");
    let source = b"NGA_TEXTURE_SOURCE_V1\nsize=2x1\nrgba=255,0,0,255;0,255,0,255\n".to_vec();
    let expected = texture_rgba_bytes(2, 1, &[255, 0, 0, 255, 0, 255, 0, 255]);
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<Texture>());
    assert_eq!(metadata.importer.as_deref(), Some("TextureImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        expected
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );
    database.save_registry().unwrap();
    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .get(id)
            .unwrap()
            .cooked_path
            .as_ref(),
        Some(&path)
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let texture: Handle<Texture> = server.load(path);
    server.update_loading();
    finish_uploads(&mut server);

    assert!(server.is_ready(&texture));
    let loaded = server.get(&texture).unwrap();
    assert_eq!((loaded.width, loaded.height), (2, 1));
    assert_eq!(loaded.data, vec![255, 0, 0, 255, 0, 255, 0, 255]);
}

#[test]
fn database_texture_importer_reports_invalid_text_source() {
    let config = database_config("texture_importer_invalid_source");
    let path = AssetPath::parse("textures/invalid.texture");
    let mut io = MemoryAssetIo::new();
    io.insert(
        path.path(),
        b"NGA_TEXTURE_SOURCE_V1\nsize=2x1\nrgba=255,0,0,255\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `TextureImporter` failed")
                && message.contains("texture source rgba byte count 4 did not match expected 8")
                && message.contains("textures/invalid.texture")
    ));
}

#[test]
fn database_builtin_mesh_import_cooks_to_binary_payload_and_runtime_loads() {
    let config = database_config("builtin_mesh_runtime_load");
    let path = AssetPath::parse("meshes/tri.mesh");
    let bytes = mesh_bytes();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(metadata.importer.as_deref(), Some("MeshImporter"));
    assert_eq!(metadata.importer_version, 4);
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    let expected_cooked = simple_binary_mesh_bytes();

    assert_eq!(output.bytes, expected_cooked);
    assert_eq!(output.version_hash, VersionHash(4));
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected_cooked
    );
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert!(metadata.cooked_hash.is_some());

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let mesh: Handle<Mesh> = server.load(path);
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].kind, GpuUploadKind::Mesh);
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(2))),
    );

    assert!(server.is_ready(&mesh));
    let loaded = server.get(&mesh).unwrap();
    assert_eq!(loaded.vertices.len(), 3);
    assert_eq!(loaded.indices, vec![0, 1, 2]);
    assert_eq!(loaded.gpu, Some(GpuResourceHandle(2)));
}

#[test]
fn database_mesh_cooker_uses_u16_indices_for_mobile_and_web_targets() {
    let config = database_config("mesh_cooker_web_u16_indices");
    let path = AssetPath::parse("meshes/tri.mesh");
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), mesh_bytes());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let windows_output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    let android_output = database.cook_asset(id, TargetPlatform::Android).unwrap();
    let ios_output = database.cook_asset(id, TargetPlatform::Ios).unwrap();
    let web_output = database.cook_asset(id, TargetPlatform::Web).unwrap();
    let expected_web = simple_u16_index_binary_mesh_bytes();

    assert_eq!(windows_output.bytes, simple_binary_mesh_bytes());
    assert_eq!(windows_output.version_hash, VersionHash(4));
    assert_eq!(android_output.bytes, expected_web);
    assert_eq!(android_output.version_hash, VersionHash(4));
    assert_eq!(ios_output.bytes, expected_web);
    assert_eq!(ios_output.version_hash, VersionHash(4));
    assert_eq!(web_output.bytes, expected_web);
    assert_eq!(web_output.version_hash, VersionHash(4));
    assert_eq!(
        web_output.bytes.len(),
        windows_output.bytes.len() - 3 * std::mem::size_of::<u16>()
    );
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected_web
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let mesh: Handle<Mesh> = server.load(path);
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    let GpuUploadMetadata::Mesh(upload_metadata) = &uploads[0].metadata else {
        panic!("u16-index cooked mesh should expose mesh upload metadata");
    };
    assert_eq!(upload_metadata.vertex_buffer_bytes, 36);
    assert_eq!(upload_metadata.index_buffer_bytes, 6);
    assert_eq!(upload_metadata.index_count, 3);
    assert_eq!(upload_metadata.index_format, MeshIndexFormat::Uint16);
    assert_eq!(uploads[0].bytes.len(), 42);
    assert_eq!(&uploads[0].bytes[36..], &[0, 0, 1, 0, 2, 0]);

    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(3))),
    );

    assert!(server.is_ready(&mesh));
    let loaded = server.get(&mesh).unwrap();
    assert_eq!(loaded.indices, vec![0, 1, 2]);
    assert_eq!(loaded.index_format, MeshIndexFormat::Uint16);
    assert_eq!(loaded.gpu_bytes(), 42);
    assert_eq!(loaded.gpu, Some(GpuResourceHandle(3)));
}

#[test]
fn database_mesh_cooker_compacts_mobile_web_vertices() {
    let config = database_config("mesh_cooker_mobile_vertex_compaction");
    let path = AssetPath::parse("meshes/unoptimized.mesh");
    let mut io = MemoryAssetIo::new();
    io.insert(
        path.path(),
        b"v 0 0 0\nv 10 0 0\nv 1 0 0\nv 0 1 0\nv 1 0 0\ni 0 4 3\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let windows_output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    let web_output = database.cook_asset(id, TargetPlatform::Web).unwrap();

    assert_eq!(windows_output.bytes, unoptimized_mesh_binary_bytes());
    assert_eq!(windows_output.version_hash, VersionHash(4));
    assert_eq!(web_output.bytes, simple_u16_index_binary_mesh_bytes());
    assert_eq!(web_output.version_hash, VersionHash(4));
    assert!(web_output.bytes.len() < windows_output.bytes.len());

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let mesh: Handle<Mesh> = server.load(path);
    server.update_loading();
    finish_uploads(&mut server);

    assert!(server.is_ready(&mesh));
    let loaded = server.get(&mesh).unwrap();
    assert_eq!(
        loaded.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(loaded.indices, vec![0, 1, 2]);
}

#[test]
fn database_mesh_cooker_canonicalizes_runtime_and_source_bytes() {
    let runtime_path = AssetPath::parse("meshes/cooked_runtime.mesh");
    let source_path = AssetPath::parse("meshes/from_source.mesh");
    let runtime_bytes = simple_binary_mesh_bytes();
    let source_bytes = mesh_bytes();
    let expected_source_bytes = simple_binary_mesh_bytes();
    let runtime_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(runtime_path.clone()),
        source_bytes: runtime_bytes.clone(),
    };
    let source_ctx = CookContext {
        target: TargetPlatform::Windows,
        source_path: Some(source_path.clone()),
        source_bytes: source_bytes.clone(),
    };
    let runtime_metadata =
        AssetMetadata::runtime(AssetId::new(), runtime_path, AssetTypeId::of::<Mesh>());
    let source_metadata =
        AssetMetadata::runtime(AssetId::new(), source_path, AssetTypeId::of::<Mesh>());
    let cooker = MeshCooker::new();

    let runtime_output = cooker.cook(&runtime_ctx, &runtime_metadata).unwrap();
    let source_output = cooker.cook(&source_ctx, &source_metadata).unwrap();

    assert_eq!(runtime_output.bytes, runtime_bytes);
    assert_eq!(runtime_output.version_hash, VersionHash(4));
    assert_eq!(runtime_output.metadata, runtime_metadata);
    assert_eq!(source_output.bytes, expected_source_bytes);
    assert_eq!(source_output.version_hash, VersionHash(4));
    assert_eq!(source_output.metadata, source_metadata);
}

#[test]
fn database_mesh_importer_canonicalizes_source_to_runtime_bytes() {
    let config = database_config("mesh_importer_source_conversion");
    let path = AssetPath::parse("meshes/generated.mesh");
    let source = b"NGA_MESH_SOURCE_V1\n# source accepts key/value and directive forms\nvertex = 0.0, 0.0, 0.0\nv 1.50 0 0\nv 0 1 0\nnormal = 0, 0, 1\nn 0 0 1\nn 0 0 1\nuv = 0, 0\nuv 1 0\nuv 0 1\nuv1 = 0.25, 0.25\nuv1 0.75 0.25\nuv1 0.25 0.75\ntangent = 1, 0, 0, 1\nt 1 0 0 1\nt 1 0 0 1\njoint = 0, 1, 2, 3\nj 0 0 0 0\njoints 1 2 3 4\nweight = 0.7, 0.2, 0.1, 0\nw 1 0 0 0\nweights 0.25 0.25 0.25 0.25\ntriangle = 0, 1, 2\n".to_vec();
    let expected = b"v 0 0 0\nv 1.5 0 0\nv 0 1 0\nn 0 0 1\nn 0 0 1\nn 0 0 1\nuv 0 0\nuv 1 0\nuv 0 1\nuv1 0.25 0.25\nuv1 0.75 0.25\nuv1 0.25 0.75\nt 1 0 0 1\nt 1 0 0 1\nt 1 0 0 1\nj 0 1 2 3\nj 0 0 0 0\nj 1 2 3 4\nw 0.7 0.2 0.1 0\nw 1 0 0 0\nw 0.25 0.25 0.25 0.25\ni 0 1 2\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(metadata.importer.as_deref(), Some("MeshImporter"));
    assert_eq!(metadata.importer_version, 4);
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        expected
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    let expected_cooked = converted_source_binary_mesh_bytes();
    assert_eq!(output.bytes, expected_cooked);
    assert_eq!(output.version_hash, VersionHash(4));
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected_cooked
    );
    database.save_registry().unwrap();
    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .get(id)
            .unwrap()
            .cooked_path
            .as_ref(),
        Some(&path)
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let mesh: Handle<Mesh> = server.load(path);
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].kind, GpuUploadKind::Mesh);
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(4))),
    );

    assert!(server.is_ready(&mesh));
    let loaded = server.get(&mesh).unwrap();
    assert_eq!(
        loaded.vertices,
        vec![[0.0, 0.0, 0.0], [1.5, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(loaded.normals, vec![[0.0, 0.0, 1.0]; 3]);
    assert_eq!(loaded.uvs, vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    assert_eq!(
        loaded.uv_sets,
        vec![vec![[0.25, 0.25], [0.75, 0.25], [0.25, 0.75]]]
    );
    assert_eq!(loaded.tangents, vec![[1.0, 0.0, 0.0, 1.0]; 3]);
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
    assert_eq!(loaded.vertex_buffer.layout.vertex_count, 3);
    assert_eq!(loaded.vertex_buffer.layout.stride, 80);
    assert_eq!(loaded.vertex_buffer.bytes.len(), 240);
    assert_eq!(loaded.indices, vec![0, 1, 2]);
    assert_eq!(loaded.gpu, Some(GpuResourceHandle(4)));
}

#[test]
fn database_mesh_importer_validates_binary_source_and_preserves_runtime_bytes() {
    let config = database_config("mesh_importer_binary_source");
    let path = AssetPath::parse("meshes/generated_binary.mesh");
    let source = binary_mesh_bytes();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<Mesh>());
    assert_eq!(metadata.importer.as_deref(), Some("MeshImporter"));
    assert_eq!(metadata.importer_version, 4);
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        source
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.bytes, binary_mesh_bytes());
    assert_eq!(output.version_hash, VersionHash(4));
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        binary_mesh_bytes()
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let mesh: Handle<Mesh> = server.load(path);
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    let GpuUploadMetadata::Mesh(upload_metadata) = &uploads[0].metadata else {
        panic!("binary mesh upload should expose mesh metadata");
    };
    assert_eq!(upload_metadata.layout.vertex_count, 3);
    assert_eq!(upload_metadata.layout.stride, 64);
    assert_eq!(upload_metadata.vertex_buffer_bytes, 192);
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(5))),
    );

    assert!(server.is_ready(&mesh));
    let loaded = server.get(&mesh).unwrap();
    assert_eq!(
        loaded.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(loaded.normals, vec![[0.0, 0.0, 1.0]; 3]);
    assert_eq!(
        loaded.uv_sets,
        vec![vec![[0.25, 0.25], [0.75, 0.25], [0.25, 0.75]]]
    );
    assert_eq!(
        loaded.joints,
        vec![[0, 1, 2, 3], [0, 0, 0, 0], [1, 2, 3, 4]]
    );
    assert_eq!(loaded.vertex_buffer.layout.stride, 64);
    assert_eq!(loaded.gpu, Some(GpuResourceHandle(5)));
}

#[test]
fn database_mesh_importer_reports_invalid_binary_source() {
    let config = database_config("mesh_importer_invalid_binary_source");
    let path = AssetPath::parse("meshes/invalid_binary.mesh");
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), invalid_binary_mesh_bytes());
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `MeshImporter` failed")
                && message.contains("mesh binary source is invalid")
                && message.contains("mesh binary index count 4 must be divisible by 3")
                && message.contains("meshes/invalid_binary.mesh")
    ));
}

#[test]
fn database_mesh_importer_reports_invalid_source_index() {
    let config = database_config("mesh_importer_invalid_source");
    let path = AssetPath::parse("meshes/invalid.mesh");
    let mut io = MemoryAssetIo::new();
    io.insert(
        path.path(),
        b"NGA_MESH_SOURCE_V1\nv 0 0 0\ni 0 1 2\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `MeshImporter` failed")
                && message.contains("mesh source index 1 references missing vertex; vertex count is 1")
                && message.contains("meshes/invalid.mesh")
    ));
}

#[test]
fn database_mesh_importer_reports_mismatched_source_attribute_counts() {
    let config = database_config("mesh_importer_invalid_attribute_count");
    let path = AssetPath::parse("meshes/invalid_attributes.mesh");
    let mut io = MemoryAssetIo::new();
    io.insert(
        path.path(),
        b"NGA_MESH_SOURCE_V1\nv 0 0 0\nv 1 0 0\nt 1 0 0 1\ni 0 1 1\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `MeshImporter` failed")
                && message.contains("mesh source tangent count 1 must match vertex count 2")
                && message.contains("meshes/invalid_attributes.mesh")
    ));
}

#[test]
fn database_mesh_importer_reports_mismatched_skinning_attribute_counts() {
    let config = database_config("mesh_importer_invalid_skinning_count");
    let path = AssetPath::parse("meshes/invalid_skin.mesh");
    let mut io = MemoryAssetIo::new();
    io.insert(
        path.path(),
        b"NGA_MESH_SOURCE_V1\nv 0 0 0\nv 1 0 0\nj 0 1 2 3\nw 1 0 0 0\ni 0 1 1\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `MeshImporter` failed")
                && message.contains("mesh source skin joint count 1 must match vertex count 2")
                && message.contains("meshes/invalid_skin.mesh")
    ));
}

#[test]
fn database_mesh_importer_reports_invalid_skin_weight_totals() {
    let invalid_binary = mesh_binary_bytes(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[],
        &[],
        &[],
        &[],
        &[
            [0u16, 1u16, 2u16, 3u16],
            [0u16, 0u16, 0u16, 0u16],
            [1u16, 2u16, 3u16, 4u16],
        ],
        &[
            [2.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.25, 0.25, 0.25, 0.25],
        ],
        &[0, 1, 2],
    );
    let cases = vec![
        (
            "mesh_importer_zero_skin_weight_total",
            "meshes/zero_skin_weight.mesh",
            b"NGA_MESH_SOURCE_V1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 1 2 3\nj 0 0 0 0\nj 1 2 3 4\nw 0 0 0 0\nw 1 0 0 0\nw 0.25 0.25 0.25 0.25\ni 0 1 2\n".to_vec(),
            "mesh source skin weight total must be positive",
        ),
        (
            "mesh_importer_unnormalized_skin_weight_total",
            "meshes/unnormalized_skin_weight.mesh",
            b"NGA_MESH_SOURCE_V1\nv 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 1 2 3\nj 0 0 0 0\nj 1 2 3 4\nw 2 0 0 0\nw 1 0 0 0\nw 0.25 0.25 0.25 0.25\ni 0 1 2\n".to_vec(),
            "mesh source skin weights on line 8 must sum to 1.0",
        ),
        (
            "mesh_importer_binary_unnormalized_skin_weight_total",
            "meshes/binary_unnormalized_skin_weight.mesh",
            invalid_binary,
            "mesh binary skin weights at vertex 0 must sum to 1.0",
        ),
    ];

    for (config_name, asset_path, source, expected_message) in cases {
        let config = database_config(config_name);
        let path = AssetPath::parse(asset_path);
        let mut io = MemoryAssetIo::new();
        io.insert(path.path(), source);
        let mut database = AssetDatabase::new(config);
        database.set_io(io);
        database.register_builtin_importers();

        let error = database.import_asset_path(&path).unwrap_err();

        assert!(matches!(
            error,
            AssetError::Import { message }
                if message.contains("importer `MeshImporter` failed")
                    && message.contains(expected_message)
                    && message.contains(asset_path)
        ));
    }
}

#[test]
fn database_builtin_shader_import_cook_and_runtime_load_preserves_payload() {
    let config = database_config("builtin_shader_runtime_load");
    let path = AssetPath::parse("shaders/pbr.wgsl");
    let bytes = shader_bytes();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<Shader>());
    assert_eq!(metadata.importer.as_deref(), Some("ShaderImporter"));
    assert_eq!(metadata.importer_version, 3);
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

    assert_eq!(output.bytes, bytes);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        bytes
    );
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert!(metadata.cooked_hash.is_some());

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let shader: Handle<Shader> = server.load(path);
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].kind, GpuUploadKind::Shader);
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(3))),
    );

    assert!(server.is_ready(&shader));
    let loaded = server.get(&shader).unwrap();
    assert_eq!(loaded.stages.len(), 1);
    assert_eq!(loaded.stages[0].stage, ShaderStage::Fragment);
    assert!(matches!(
        &loaded.stages[0].source,
        ShaderSource::Wgsl(source) if source == "@fragment fn main() {}\n"
    ));
    assert_eq!(loaded.gpu, Some(GpuResourceHandle(3)));
}

#[test]
fn database_shader_importer_canonicalizes_source_to_runtime_wgsl() {
    let config = database_config("shader_importer_source_conversion");
    let path = AssetPath::parse("shaders/generated.wgsl");
    let source =
        b"NGA_SHADER_SOURCE_V1\nlanguage=wgsl\nstage=fragment\n---\n  @fragment fn main() {}\n"
            .to_vec();
    let expected = b"@fragment fn main() {}\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<Shader>());
    assert_eq!(metadata.importer.as_deref(), Some("ShaderImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        expected
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );
    database.save_registry().unwrap();
    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .get(id)
            .unwrap()
            .cooked_path
            .as_ref(),
        Some(&path)
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let shader: Handle<Shader> = server.load(path);
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].kind, GpuUploadKind::Shader);
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(5))),
    );

    assert!(server.is_ready(&shader));
    let loaded = server.get(&shader).unwrap();
    assert_eq!(loaded.stages[0].stage, ShaderStage::Fragment);
    assert!(matches!(
        &loaded.stages[0].source,
        ShaderSource::Wgsl(source) if source == "@fragment fn main() {}\n"
    ));
    assert_eq!(loaded.gpu, Some(GpuResourceHandle(5)));
}

#[test]
fn database_shader_importer_preserves_glsl_source_language() {
    let config = database_config("shader_importer_glsl_source_conversion");
    let path = AssetPath::parse("shaders/generated.glsl");
    let source = b"NGA_SHADER_SOURCE_V1\nlanguage=glsl\nstage=vertex\n---\n#version 450\nlayout(location = 0) in vec3 position;\nvoid main() {}\n".to_vec();
    let expected =
        b"#version 450\nlayout(location = 0) in vec3 position;\nvoid main() {}\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<Shader>());
    assert_eq!(metadata.importer.as_deref(), Some("ShaderImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        expected
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let shader: Handle<Shader> = server.load(path);
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].kind, GpuUploadKind::Shader);
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(4))),
    );

    assert!(server.is_ready(&shader));
    let loaded = server.get(&shader).unwrap();
    assert_eq!(loaded.stages[0].stage, ShaderStage::Fragment);
    assert!(matches!(
        &loaded.stages[0].source,
        ShaderSource::Glsl(source)
            if source == "#version 450\nlayout(location = 0) in vec3 position;\nvoid main() {}\n"
    ));
    assert!(loaded.reflection.is_none());
}

#[test]
fn database_shader_importer_preserves_spv_source_language() {
    let config = database_config("shader_importer_spv_source_conversion");
    let path = AssetPath::parse("shaders/generated.spv");
    let source = b"NGA_SHADER_SOURCE_V1\nlanguage=spv\nstage=compute\nsource=0x07230203,0x00010000,0x00000000,0x00000000\n".to_vec();
    let expected = {
        let mut bytes = Vec::new();
        for word in [0x0723_0203u32, 0x0001_0000, 0x0000_0000, 0x0000_0000] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes
    };
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<Shader>());
    assert_eq!(metadata.importer.as_deref(), Some("ShaderImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        expected
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let shader: Handle<Shader> = server.load(AssetPath::with_label(path.path(), "compute"));
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].kind, GpuUploadKind::Shader);
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(6))),
    );

    assert!(server.is_ready(&shader));
    let loaded = server.get(&shader).unwrap();
    assert_eq!(loaded.stages[0].stage, ShaderStage::Compute);
    assert!(matches!(
        &loaded.stages[0].source,
        ShaderSource::Spirv(words)
            if words.as_slice()
                == [0x0723_0203, 0x0001_0000, 0x0000_0000, 0x0000_0000]
    ));
    assert!(loaded.reflection.is_none());
}

#[test]
fn database_shader_importer_reports_invalid_empty_source() {
    let config = database_config("shader_importer_invalid_source");
    let path = AssetPath::parse("shaders/invalid.wgsl");
    let mut io = MemoryAssetIo::new();
    io.insert(
        path.path(),
        b"NGA_SHADER_SOURCE_V1\nlanguage=wgsl\nsource=   \n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ShaderImporter` failed")
                && message.contains("shader source body is empty")
                && message.contains("shaders/invalid.wgsl")
    ));
}

#[test]
fn database_shader_importer_validates_entry_presence() {
    let config = database_config("shader_importer_missing_entry");
    let path = AssetPath::parse("shaders/missing_entry.wgsl");
    let source =
        b"NGA_SHADER_SOURCE_V1\nlanguage=wgsl\nentry=main\nsource=@fragment fn no_entry() {}\n"
            .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source);
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `ShaderImporter` failed")
                && message.contains("shader source entry `main` is not defined in source body")
                && message.contains(path.path())
    ));
}

#[test]
fn database_shader_importer_validates_stage_and_entry_metadata() {
    for (config_name, path, source, expected_message) in [
        (
            "shader_importer_invalid_stage",
            "shaders/invalid_stage.wgsl",
            b"NGA_SHADER_SOURCE_V1\nlanguage=wgsl\nstage=geometry\nsource=@fragment fn main() {}\n"
                .to_vec(),
            "unsupported shader source stage `geometry` on line 3",
        ),
        (
            "shader_importer_invalid_entry",
            "shaders/invalid_entry.wgsl",
            b"NGA_SHADER_SOURCE_V1\nlanguage=wgsl\nentry=main-entry\nsource=@fragment fn main() {}\n"
                .to_vec(),
            "invalid shader source entry `main-entry` on line 3",
        ),
    ] {
        let config = database_config(config_name);
        let path = AssetPath::parse(path);
        let mut io = MemoryAssetIo::new();
        io.insert(path.path(), source);
        let mut database = AssetDatabase::new(config);
        database.set_io(io);
        database.register_builtin_importers();

        let error = database.import_asset_path(&path).unwrap_err();

        assert!(matches!(
            error,
            AssetError::Import { message }
                if message.contains("importer `ShaderImporter` failed")
                    && message.contains(expected_message)
                    && message.contains(path.path())
        ));
    }
}

#[test]
fn database_shader_importer_reports_wgsl_compile_validation_failure() {
    let config = database_config("shader_importer_invalid_wgsl");
    let path = AssetPath::parse("shaders/invalid_syntax.wgsl");
    let source =
        b"NGA_SHADER_SOURCE_V1\nlanguage=wgsl\nstage=fragment\n---\n@fragment fn main() { let x = 1 + ; }\n"
            .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source);
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message } if message.contains("importer `ShaderImporter` failed")
            && (message.contains("WGSL compile failed")
                || message.contains("WGSL validation failed"))
            && message.contains(path.path())
            && message.contains("line 1, column")
    ));
}

#[test]
fn database_shader_importer_case_insensitive_keys_and_duplicate_rejection() {
    let config = database_config("shader_importer_case_insensitive_source");
    let path = AssetPath::parse("shaders/case_insensitive.glsl");
    let source =
        b"NGA_SHADER_SOURCE_V1\nLANGUAGE=GLSL\nStAgE=vertex\nENTRY=Main\n---\n#version 450\nlayout(location = 0) in vec3 position;\nvoid main() {}\n"
            .to_vec();
    let expected =
        b"#version 450\nlayout(location = 0) in vec3 position;\nvoid main() {}\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.importer.as_deref(), Some("ShaderImporter"));
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        expected
    );

    let error_paths = [
        (
            "shader_importer_duplicate_language",
            "shaders/duplicate_language.glsl",
            b"NGA_SHADER_SOURCE_V1\nlanguage=glsl\nlanguage=wgsl\nsource=0\n".to_vec(),
            "shader source key `language` is repeated on line 3",
        ),
        (
            "shader_importer_duplicate_stage",
            "shaders/duplicate_stage.glsl",
            b"NGA_SHADER_SOURCE_V1\nlanguage=glsl\nstage=vertex\nstage=compute\nsource=0".to_vec(),
            "shader source key `stage` is repeated on line 4",
        ),
        (
            "shader_importer_duplicate_entry",
            "shaders/duplicate_entry.glsl",
            b"NGA_SHADER_SOURCE_V1\nlanguage=glsl\nentry=main\nentry=main\nsource=0".to_vec(),
            "shader source key `entry` is repeated on line 4",
        ),
        (
            "shader_importer_duplicate_source",
            "shaders/duplicate_source.glsl",
            b"NGA_SHADER_SOURCE_V1\nlanguage=glsl\nsource=0\nsource=1".to_vec(),
            "shader source body is repeated on line 4",
        ),
    ];
    for (config_name, path, source, expected_message) in error_paths {
        let config = database_config(config_name);
        let path = AssetPath::parse(path);
        let mut io = MemoryAssetIo::new();
        io.insert(path.path(), source);
        let mut database = AssetDatabase::new(config);
        database.set_io(io);
        database.register_builtin_importers();

        let error = database.import_asset_path(&path).unwrap_err();
        assert!(matches!(
            error,
            AssetError::Import { message }
                if message.contains("importer `ShaderImporter` failed")
                    && message.contains(expected_message)
                    && message.contains(path.path())
        ));
    }
}

#[test]
fn database_builtin_audio_import_cook_and_runtime_load_preserves_payload() {
    let config = database_config("builtin_audio_runtime_load");
    let path = AssetPath::parse("audio/click.audio");
    let bytes = audio_bytes();
    let expected = b"NGA_AUDIO_V1\nsample_rate=44100\nchannels=1\nformat=f32\nsamples=0,0.5,-0.5\nstreaming=false\n"
        .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<AudioClip>());
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert!(metadata.cooked_hash.is_some());

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let audio: Handle<AudioClip> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&audio));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&audio).unwrap();
    assert_eq!(loaded.sample_rate, 44100);
    assert_eq!(loaded.channels, 1);
    assert_eq!(loaded.samples, AudioSamples::F32(vec![0.0, 0.5, -0.5]));
}

#[test]
fn database_builtin_wav_audio_import_cook_and_runtime_load_preserves_payload() {
    let config = database_config("builtin_wav_audio_runtime_load");
    let path = AssetPath::parse("audio/click.wav");
    let bytes = wav_pcm16_bytes(44_100, 2, &[0, 1000, -1000, 500]);
    let expected = b"NGA_AUDIO_V1\nsample_rate=44100\nchannels=2\nformat=i16\nsamples=0,1000,-1000,500\nstreaming=false\n"
        .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<AudioClip>());
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        bytes
    );
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert!(metadata.cooked_hash.is_some());

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let audio: Handle<AudioClip> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&audio));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&audio).unwrap();
    assert_eq!(loaded.sample_rate, 44_100);
    assert_eq!(loaded.channels, 2);
    assert_eq!(loaded.duration_seconds, 2.0 / 44_100.0);
    assert_eq!(loaded.samples, AudioSamples::I16(vec![0, 1000, -1000, 500]));
    assert!(!loaded.streaming);
}

#[test]
fn database_builtin_wav_pcm24_import_cook_and_runtime_load_canonicalizes_to_i16() {
    let config = database_config("builtin_wav_pcm24_audio_runtime_load");
    let path = AssetPath::parse("audio/hit.wav");
    let bytes = wav_pcm_bytes(
        44_100,
        2,
        24,
        &[
            0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0xff, 0xff, 0x7f, 0x00, 0xff, 0xff,
        ],
    );
    let expected = b"NGA_AUDIO_V1\nsample_rate=44100\nchannels=2\nformat=i16\nsamples=-32768,0,32767,-1\nstreaming=false\n"
        .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<AudioClip>());
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        bytes
    );
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

    assert_eq!(output.version_hash, VersionHash(8));
    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert!(metadata.cooked_hash.is_some());

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let audio: Handle<AudioClip> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&audio));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&audio).unwrap();
    assert_eq!(loaded.sample_rate, 44_100);
    assert_eq!(loaded.channels, 2);
    assert_eq!(loaded.duration_seconds, 2.0 / 44_100.0);
    assert_eq!(
        loaded.samples,
        AudioSamples::I16(vec![-32768, 0, 32767, -1])
    );
    assert!(!loaded.streaming);
}

#[test]
fn database_builtin_wav_g711_import_cook_and_runtime_loads() {
    let cases = [
        (
            "alaw",
            6u16,
            vec![0xd5, 0x55, 0xaa, 0x2a],
            vec![8, -8, 32256, -32256],
            "8,-8,32256,-32256",
        ),
        (
            "mulaw",
            7u16,
            vec![0xff, 0x7f, 0x80, 0x00],
            vec![0, 0, 32124, -32124],
            "0,0,32124,-32124",
        ),
    ];

    for (name, audio_format, encoded_samples, expected_samples, expected_sample_text) in cases {
        let config = database_config(&format!("builtin_wav_{name}_g711_audio_runtime_load"));
        let path = AssetPath::parse(&format!("audio/{name}.wav"));
        let bytes = wav_format_bytes(44_100, 2, audio_format, 8, &encoded_samples);
        let expected = format!(
            "NGA_AUDIO_V1\nsample_rate=44100\nchannels=2\nformat=i16\nsamples={expected_sample_text}\nstreaming=false\n"
        )
        .into_bytes();
        let mut io = MemoryAssetIo::new();
        io.insert(path.path(), bytes.clone());
        let mut database = AssetDatabase::new(config.clone());
        database.set_io(io);
        database.register_builtin_importers();
        database.register_builtin_cookers();

        let id = database.import_asset_path(&path).unwrap();
        let metadata = database.registry().get(id).unwrap();
        assert_eq!(metadata.asset_type, AssetTypeId::of::<AudioClip>());
        assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
        assert_eq!(metadata.importer_version, 3);
        assert_eq!(
            fs::read(config.imported_root.join(path.path())).unwrap(),
            bytes
        );
        let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

        assert_eq!(output.version_hash, VersionHash(8));
        assert_eq!(output.bytes, expected);
        assert_eq!(
            fs::read(config.cooked_root.join(path.path())).unwrap(),
            expected
        );
        let metadata = database.registry().get(id).unwrap();
        assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
        assert!(metadata.cooked_hash.is_some());

        let mut server = AssetServer::new(AssetServerConfig {
            root: config.cooked_root.clone(),
            ..AssetServerConfig::default()
        });
        server.register_builtin_loaders();
        let audio: Handle<AudioClip> = server.load(path);
        server.update_loading();

        assert!(server.is_ready(&audio), "{name} should load");
        assert!(server.drain_gpu_uploads().next().is_none());
        let loaded = server.get(&audio).unwrap();
        assert_eq!(loaded.sample_rate, 44_100);
        assert_eq!(loaded.channels, 2);
        assert_eq!(loaded.duration_seconds, 2.0 / 44_100.0);
        assert_eq!(loaded.samples, AudioSamples::I16(expected_samples));
        assert!(!loaded.streaming);
    }
}

#[test]
fn database_builtin_wav_ima_adpcm_import_cook_and_runtime_loads() {
    let config = database_config("builtin_wav_ima_adpcm_audio_runtime_load");
    let path = AssetPath::parse("audio/voice.wav");
    let block = [0, 0, 0, 0, 0x11, 0x91, 0, 0];
    let bytes = wav_ima_adpcm_bytes(22_050, 1, 8, 5, &block);
    let expected =
        b"NGA_AUDIO_V1\nsample_rate=22050\nchannels=1\nformat=i16\nsamples=0,1,2,3,2\nstreaming=false\n"
            .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<AudioClip>());
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        bytes
    );
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

    assert_eq!(output.version_hash, VersionHash(8));
    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert!(metadata.cooked_hash.is_some());

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let audio: Handle<AudioClip> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&audio));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&audio).unwrap();
    assert_eq!(loaded.sample_rate, 22_050);
    assert_eq!(loaded.channels, 1);
    assert_eq!(loaded.duration_seconds, 5.0 / 22_050.0);
    assert_eq!(loaded.samples, AudioSamples::I16(vec![0, 1, 2, 3, 2]));
    assert!(!loaded.streaming);
}

#[test]
fn database_builtin_wav_ms_adpcm_import_cook_and_runtime_loads() {
    let config = database_config("builtin_wav_ms_adpcm_audio_runtime_load");
    let path = AssetPath::parse("audio/radio.wav");
    let mut block = Vec::new();
    block.push(0);
    block.extend_from_slice(&16i16.to_le_bytes());
    block.extend_from_slice(&1000i16.to_le_bytes());
    block.extend_from_slice(&990i16.to_le_bytes());
    block.push(0x11);
    let bytes = wav_ms_adpcm_bytes(22_050, 1, 8, 4, &block);
    let expected =
        b"NGA_AUDIO_V1\nsample_rate=22050\nchannels=1\nformat=i16\nsamples=990,1000,1016,1032\nstreaming=false\n"
            .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<AudioClip>());
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        bytes
    );
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

    assert_eq!(output.version_hash, VersionHash(8));
    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert!(metadata.cooked_hash.is_some());

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let audio: Handle<AudioClip> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&audio));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&audio).unwrap();
    assert_eq!(loaded.sample_rate, 22_050);
    assert_eq!(loaded.channels, 1);
    assert_eq!(loaded.duration_seconds, 4.0 / 22_050.0);
    assert_eq!(
        loaded.samples,
        AudioSamples::I16(vec![990, 1000, 1016, 1032])
    );
    assert!(!loaded.streaming);
}

#[test]
fn database_builtin_wav_extensible_encoded_import_cook_and_runtime_loads() {
    let cases = [
        (
            "alaw",
            wav_extensible_bytes(44_100, 2, 8, 6, &[0xd5, 0x55, 0xaa, 0x2a]),
            b"NGA_AUDIO_V1\nsample_rate=44100\nchannels=2\nformat=i16\nsamples=8,-8,32256,-32256\nstreaming=false\n"
                .to_vec(),
            44_100,
            2,
            2.0 / 44_100.0,
            vec![8, -8, 32256, -32256],
        ),
        (
            "ima",
            wav_extensible_ima_adpcm_bytes(22_050, 1, 8, 5, &[0, 0, 0, 0, 0x11, 0x91, 0, 0]),
            b"NGA_AUDIO_V1\nsample_rate=22050\nchannels=1\nformat=i16\nsamples=0,1,2,3,2\nstreaming=false\n"
                .to_vec(),
            22_050,
            1,
            5.0 / 22_050.0,
            vec![0, 1, 2, 3, 2],
        ),
    ];

    for (name, bytes, expected, sample_rate, channels, duration_seconds, expected_samples) in cases
    {
        let config = database_config(&format!(
            "builtin_wav_extensible_{name}_encoded_audio_runtime_load"
        ));
        let path = AssetPath::parse(&format!("audio/{name}.wav"));
        let mut io = MemoryAssetIo::new();
        io.insert(path.path(), bytes.clone());
        let mut database = AssetDatabase::new(config.clone());
        database.set_io(io);
        database.register_builtin_importers();
        database.register_builtin_cookers();

        let id = database.import_asset_path(&path).unwrap();
        let metadata = database.registry().get(id).unwrap();
        assert_eq!(metadata.asset_type, AssetTypeId::of::<AudioClip>());
        assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
        assert_eq!(metadata.importer_version, 3);
        assert_eq!(
            fs::read(config.imported_root.join(path.path())).unwrap(),
            bytes
        );
        let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

        assert_eq!(output.version_hash, VersionHash(8));
        assert_eq!(output.bytes, expected);
        assert_eq!(
            fs::read(config.cooked_root.join(path.path())).unwrap(),
            expected
        );

        let mut server = AssetServer::new(AssetServerConfig {
            root: config.cooked_root.clone(),
            ..AssetServerConfig::default()
        });
        server.register_builtin_loaders();
        let audio: Handle<AudioClip> = server.load(path);
        server.update_loading();

        assert!(server.is_ready(&audio), "{name} should load");
        assert!(server.drain_gpu_uploads().next().is_none());
        let loaded = server.get(&audio).unwrap();
        assert_eq!(loaded.sample_rate, sample_rate);
        assert_eq!(loaded.channels, channels);
        assert_eq!(loaded.duration_seconds, duration_seconds);
        assert_eq!(loaded.samples, AudioSamples::I16(expected_samples));
        assert!(!loaded.streaming);
    }
}

#[test]
fn database_builtin_wav_extensible_float_import_cook_and_runtime_loads() {
    let config = database_config("builtin_wav_extensible_float_audio_runtime_load");
    let path = AssetPath::parse("audio/pad.wav");
    let mut sample_bytes = Vec::new();
    for sample in [0.0f32, 0.5, -0.25, 1.0] {
        sample_bytes.extend_from_slice(&sample.to_le_bytes());
    }
    let bytes = wav_extensible_bytes(48_000, 2, 32, 3, &sample_bytes);
    let expected = b"NGA_AUDIO_V1\nsample_rate=48000\nchannels=2\nformat=f32\nsamples=0,0.5,-0.25,1\nstreaming=false\n"
        .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<AudioClip>());
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        bytes
    );
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

    assert_eq!(output.version_hash, VersionHash(8));
    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert!(metadata.cooked_hash.is_some());

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let audio: Handle<AudioClip> = server.load(path);
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
fn database_builtin_wav_extensible_import_cook_reports_invalid_subformat() {
    let config = database_config("builtin_wav_extensible_invalid_subformat");
    let path = AssetPath::parse("audio/unsupported.wav");
    let mut sample_bytes = Vec::new();
    for sample in [0i16, 1000, -1000, 0] {
        sample_bytes.extend_from_slice(&sample.to_le_bytes());
    }
    let bytes = wav_extensible_bytes(44_100, 2, 16, 99, &sample_bytes);
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes);
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let error = database
        .cook_asset(id, TargetPlatform::Windows)
        .unwrap_err();

    assert!(matches!(
        error,
        AssetError::Cook { message }
            if message.contains("AudioCooker failed to canonicalize audio source")
                && message.contains("unsupported WAV extensible subformat 99")
    ));
}

#[test]
fn database_audio_importer_converts_source_to_runtime_audio_bytes() {
    let config = database_config("audio_importer_source_conversion");
    let path = AssetPath::parse("audio/generated.audio");
    let source = b"NGA_AUDIO_SOURCE_V1\nsample_rate=48000\nchannels=2\nformat=f32\nframes=0.0, 0.5; -0.5, 1.0\nstreaming=true\n".to_vec();
    let expected = b"NGA_AUDIO_V1\nsample_rate=48000\nchannels=2\nformat=f32\nsamples=0,0.5,-0.5,1\nstreaming=true\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<AudioClip>());
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        expected
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );
    database.save_registry().unwrap();
    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .get(id)
            .unwrap()
            .cooked_path
            .as_ref(),
        Some(&path)
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let audio: Handle<AudioClip> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&audio));
    let loaded = server.get(&audio).unwrap();
    assert_eq!(loaded.sample_rate, 48000);
    assert_eq!(loaded.channels, 2);
    assert!(loaded.streaming);
    assert_eq!(loaded.samples, AudioSamples::F32(vec![0.0, 0.5, -0.5, 1.0]));
}

#[test]
fn database_audio_importer_preserves_binary_wav_bytes() {
    let config = database_config("audio_importer_binary_wav_round_trip");
    let path = AssetPath::parse("audio/roundtrip.wav");
    let source = wav_pcm16_bytes(44_100, 2, &[0, 1000, -1000, 500]);
    let expected = b"NGA_AUDIO_V1\nsample_rate=44100\nchannels=2\nformat=i16\nsamples=0,1000,-1000,500\nstreaming=false\n"
        .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<AudioClip>());
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        source
    );

    database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(fs::read(config.cooked_root.join(path.path())).unwrap(), expected);

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let audio: Handle<AudioClip> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&audio));
    let loaded = server.get(&audio).unwrap();
    assert_eq!(loaded.sample_rate, 44_100);
    assert_eq!(loaded.channels, 2);
    assert!(!loaded.streaming);
    assert_eq!(loaded.samples, AudioSamples::I16(vec![0, 1000, -1000, 500]));
}

#[test]
fn database_audio_importer_applies_force_mono_and_normalize_settings() {
    let config = database_config("audio_importer_force_mono_normalize");
    let path = AssetPath::parse("audio/processed.audio");
    let source = b"NGA_AUDIO_SOURCE_V1\nsample_rate=48000\nchannels=2\nformat=f32\nframes=0.0, 0.0; 0.25, 0.25; -0.5, -0.5\nstreaming=false\n".to_vec();
    let expected =
        b"NGA_AUDIO_V1\nsample_rate=48000\nchannels=1\nformat=f32\nsamples=0,0.5,-1\nstreaming=false\n"
            .to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();
    let mut settings = ImporterSettings::default();
    settings.set("force_mono", "true");
    settings.set("normalize", "true");

    let id = database
        .import_asset_path_with_settings(&path, &settings)
        .unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<AudioClip>());
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(
        metadata.importer_settings,
        vec![
            ("force_mono".to_owned(), "true".to_owned()),
            ("normalize".to_owned(), "true".to_owned())
        ]
    );
    assert!(metadata.settings_hash.is_some());
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        expected
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.bytes, expected);
    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let audio: Handle<AudioClip> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&audio));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&audio).unwrap();
    assert_eq!(loaded.sample_rate, 48000);
    assert_eq!(loaded.channels, 1);
    assert_eq!(loaded.duration_seconds, 3.0 / 48000.0);
    assert!(!loaded.streaming);
    assert_eq!(loaded.samples, AudioSamples::F32(vec![0.0, 0.5, -1.0]));
}

#[test]
fn database_audio_importer_reports_invalid_audio_settings() {
    let config = database_config("audio_importer_invalid_settings");
    let path = AssetPath::parse("audio/invalid_settings.audio");
    let mut io = MemoryAssetIo::new();
    io.insert(
        path.path(),
        b"NGA_AUDIO_SOURCE_V1\nsample_rate=48000\nchannels=1\nformat=i16\nsamples=0,1\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();
    let mut settings = ImporterSettings::default();
    settings.set("force_mono", "maybe");

    let error = database
        .import_asset_path_with_settings(&path, &settings)
        .unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `AudioImporter` failed")
                && message.contains("audio/invalid_settings.audio")
                && message.contains("force_mono=maybe")
                && message.contains(
                    "invalid audio import setting `force_mono` value `maybe`; expected true or false"
                )
    ));
}

#[test]
fn database_audio_importer_reports_invalid_audio_compression_setting() {
    let config = database_config("audio_importer_invalid_compression_setting");
    let path = AssetPath::parse("audio/invalid_compression.audio");
    let mut io = MemoryAssetIo::new();
    io.insert(
        path.path(),
        b"NGA_AUDIO_SOURCE_V1\nsample_rate=48000\nchannels=1\nformat=i16\nsamples=0,1\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();
    let mut settings = ImporterSettings::default();
    settings.set("compression", "flac");

    let error = database
        .import_asset_path_with_settings(&path, &settings)
        .unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `AudioImporter` failed")
                && message.contains("audio/invalid_compression.audio")
                && message.contains("compression=flac")
                && message.contains(
                    "invalid audio import setting `compression` value `flac`; expected none, vorbis, or opus"
                )
    ));
}

#[test]
fn database_audio_importer_applies_streaming_setting_override() {
    let config = database_config("audio_importer_streaming_override");
    let path = AssetPath::parse("audio/override.audio");
    let source = b"NGA_AUDIO_SOURCE_V1\nsample_rate=48000\nchannels=1\nformat=f32\nsamples=0.0,0.5,-0.5\nstreaming=true\n".to_vec();
    let expected = b"NGA_AUDIO_V1\nsample_rate=48000\nchannels=1\nformat=f32\nsamples=0,0.5,-0.5\nstreaming=false\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();
    let mut settings = ImporterSettings::default();
    settings.set("streaming", "false");
    settings.set("compression", "none");

    let id = database
        .import_asset_path_with_settings(&path, &settings)
        .unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(
        metadata.importer_settings,
        vec![
            ("compression".to_owned(), "none".to_owned()),
            ("streaming".to_owned(), "false".to_owned()),
        ]
    );
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        expected
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.bytes, expected);

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let audio: Handle<AudioClip> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&audio));
    let loaded = server.get(&audio).unwrap();
    assert!(!loaded.streaming);
    assert_eq!(loaded.samples, AudioSamples::F32(vec![0.0, 0.5, -0.5]));
}

#[test]
fn database_audio_importer_reports_unsupported_audio_compression() {
    let config = database_config("audio_importer_unsupported_compression");
    let path = AssetPath::parse("audio/unsupported.audio");
    let mut io = MemoryAssetIo::new();
    io.insert(
        path.path(),
        b"NGA_AUDIO_SOURCE_V1\nsample_rate=48000\nchannels=1\nformat=i16\nsamples=0,1\nstreaming=false\n"
            .to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();
    let mut settings = ImporterSettings::default();
    settings.set("compression", "vorbis");

    let error = database
        .import_asset_path_with_settings(&path, &settings)
        .unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `AudioImporter` failed")
                && message.contains("audio/unsupported.audio")
                && message.contains("compression=vorbis")
                && message.contains("unsupported audio import compression `vorbis`; expected `none`")
    ));
}

#[test]
fn database_audio_importer_allows_ogg_vorbis_compression_for_binary_source() {
    let config = database_config("audio_importer_ogg_vorbis");
    let path = AssetPath::parse("audio/callout.ogg");
    let source = ogg_vorbis_audio_bytes(44_100, 2);

    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let mut settings = ImporterSettings::default();
    settings.set("compression", "vorbis");

    let id = database
        .import_asset_path_with_settings(&path, &settings)
        .unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(
        metadata.importer_settings,
        vec![("compression".to_owned(), "vorbis".to_owned())]
    );
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        source
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.bytes, source);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        source
    );
}

#[test]
fn database_audio_importer_allows_ogg_opus_compression_for_binary_source() {
    let config = database_config("audio_importer_ogg_opus");
    let path = AssetPath::parse("audio/dialogue.ogg");
    let source = ogg_opus_audio_bytes(48_000, 1);

    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let mut settings = ImporterSettings::default();
    settings.set("compression", "opus");

    let id = database
        .import_asset_path_with_settings(&path, &settings)
        .unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(
        metadata.importer_settings,
        vec![("compression".to_owned(), "opus".to_owned())]
    );
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        source
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.bytes, source);
}

#[test]
fn database_audio_importer_rejects_ogg_compression_for_non_ogg_binary_source() {
    let config = database_config("audio_importer_ogg_binary_source_validation");
    let path = AssetPath::parse("audio/raw.audio");
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), vec![0x00, 0x01, 0x02]);
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();
    let mut settings = ImporterSettings::default();
    settings.set("compression", "vorbis");

    let error = database
        .import_asset_path_with_settings(&path, &settings)
        .unwrap_err();
    if let AssetError::Import { message } = &error {
        eprintln!("actual error: {message}");
    }

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `AudioImporter` failed")
                && message.contains("audio/raw.audio")
                && message.contains("compression=vorbis")
    ));
}

#[test]
fn database_audio_importer_rejects_ogg_compression_for_binary_wav_source() {
    let config = database_config("audio_importer_ogg_wav_source_validation");
    let path = AssetPath::parse("audio/raw.wav");
    let source = wav_pcm16_bytes(44_100, 1, &[0, 1000, -1000, 0]);
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source);
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();
    let mut settings = ImporterSettings::default();
    settings.set("compression", "opus");

    let error = database
        .import_asset_path_with_settings(&path, &settings)
        .unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `AudioImporter` failed")
                && message.contains("audio/raw.wav")
                && message.contains("compression=opus")
                && message.contains("supported Ogg payload")
    ));
}

#[test]
fn database_ogg_audio_import_cook_and_runtime_load_preserves_payload() {
    let config = database_config("builtin_ogg_audio_runtime_load");
    let path = AssetPath::parse("audio/callout.ogg");
    let source = ogg_vorbis_audio_bytes(44_100, 2);

    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<AudioClip>());
    assert_eq!(metadata.importer.as_deref(), Some("AudioImporter"));
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        source
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.bytes, source);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        source
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let audio: Handle<AudioClip> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&audio));
    let loaded = server.get(&audio).unwrap();
    assert_eq!(loaded.sample_rate, 44_100);
    assert_eq!(loaded.channels, 2);
    assert_eq!(loaded.duration_seconds, 0.0);
    assert!(loaded.streaming);
    assert!(matches!(loaded.samples, AudioSamples::Streaming(_)));
}

#[test]
fn database_audio_importer_reports_invalid_source_samples() {
    let config = database_config("audio_importer_invalid_source");
    let path = AssetPath::parse("audio/invalid.audio");
    let mut io = MemoryAssetIo::new();
    io.insert(
        path.path(),
        b"NGA_AUDIO_SOURCE_V1\nsample_rate=48000\nchannels=2\nformat=i16\nsamples=1,2,3\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `AudioImporter` failed")
                && message.contains("audio source sample count 3 must be a non-zero multiple of channels 2")
                && message.contains("audio/invalid.audio")
    ));
}

#[test]
fn database_builtin_font_import_cook_and_runtime_load_preserves_payload() {
    let config = database_config("builtin_font_runtime_load");
    let path = AssetPath::parse("fonts/debug.font");
    let bytes = font_bytes();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<Font>());
    assert_eq!(metadata.importer.as_deref(), Some("FontImporter"));
    assert_eq!(metadata.importer_version, 3);
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

    assert_eq!(output.version_hash, VersionHash(2));
    assert_eq!(output.bytes, bytes);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        bytes
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let font: Handle<Font> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&font));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&font).unwrap();
    assert_eq!(loaded.family_name, "Debug Sans");
    assert!(matches!(&loaded.data, FontData::Bitmap(bitmap) if bitmap.glyphs[0].codepoint == 'A'));
}

#[test]
fn database_builtin_binary_font_import_cook_and_runtime_loads() {
    let cases = [
        (
            "truetype",
            AssetPath::parse("fonts/interface.ttf"),
            ttf_font_bytes(),
            "interface",
        ),
        (
            "opentype",
            AssetPath::parse("fonts/display.otf"),
            otf_font_bytes(),
            "display",
        ),
    ];

    for (name, path, bytes, family_name) in cases {
        let config = database_config(&format!("builtin_{name}_font_runtime_load"));
        let mut io = MemoryAssetIo::new();
        io.insert(path.path(), bytes.clone());
        let mut database = AssetDatabase::new(config.clone());
        database.set_io(io);
        database.register_builtin_importers();
        database.register_builtin_cookers();

        let id = database.import_asset_path(&path).unwrap();
        let metadata = database.registry().get(id).unwrap();
        assert_eq!(metadata.asset_type, AssetTypeId::of::<Font>());
        assert_eq!(metadata.importer.as_deref(), Some("FontImporter"));
        assert_eq!(metadata.importer_version, 3);
        assert_eq!(
            fs::read(config.imported_root.join(path.path())).unwrap(),
            bytes
        );
        let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

        assert_eq!(output.version_hash, VersionHash(2));
        assert_eq!(output.bytes, bytes);
        assert_eq!(
            fs::read(config.cooked_root.join(path.path())).unwrap(),
            bytes
        );

        let mut server = AssetServer::new(AssetServerConfig {
            root: config.cooked_root.clone(),
            ..AssetServerConfig::default()
        });
        server.register_builtin_loaders();
        let font: Handle<Font> = server.load(path);
        server.update_loading();

        assert!(server.is_ready(&font), "{name} font should load");
        assert!(server.drain_gpu_uploads().next().is_none());
        let loaded = server.get(&font).unwrap();
        assert_eq!(loaded.family_name, family_name);
        match (&loaded.data, name) {
            (FontData::TrueType(loaded_bytes), "truetype") => assert_eq!(loaded_bytes, &bytes),
            (FontData::OpenType(loaded_bytes), "opentype") => assert_eq!(loaded_bytes, &bytes),
            _ => panic!("{name} font loaded wrong data variant"),
        }
    }
}

#[test]
fn database_font_importer_canonicalizes_source_to_runtime_font_bytes() {
    let config = database_config("font_importer_source_conversion");
    let path = AssetPath::parse("fonts/generated.font");
    let source = b"NGA_FONT_SOURCE_V1\n# canonical order should not depend on source order\nglyph = char=B; size=1x1; bitmap=128\nfamily = Debug Sans\nglyph=char=A;size=2x1;bitmap=0, 255\n".to_vec();
    let expected = b"NGA_FONT_V1\nfamily=Debug Sans\nglyph=char=A;size=2x1;bitmap=0,255\nglyph=char=B;size=1x1;bitmap=128\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<Font>());
    assert_eq!(metadata.importer.as_deref(), Some("FontImporter"));
    assert_eq!(metadata.importer_version, 3);
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        expected
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.version_hash, VersionHash(2));
    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );
    database.save_registry().unwrap();
    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .get(id)
            .unwrap()
            .cooked_path
            .as_ref(),
        Some(&path)
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let font: Handle<Font> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&font));
    let loaded = server.get(&font).unwrap();
    assert_eq!(loaded.family_name, "Debug Sans");
    let FontData::Bitmap(bitmap) = &loaded.data else {
        panic!("font source importer should produce bitmap font bytes");
    };
    assert_eq!(bitmap.glyphs.len(), 2);
    assert_eq!(bitmap.glyphs[0].codepoint, 'A');
    assert_eq!(bitmap.glyphs[0].bitmap, vec![0, 255]);
    assert_eq!(bitmap.glyphs[1].codepoint, 'B');
    assert_eq!(bitmap.glyphs[1].bitmap, vec![128]);
}

#[test]
fn database_font_importer_preserves_binary_font_bytes() {
    let config = database_config("font_importer_binary_font_round_trip");
    let truetype_path = AssetPath::parse("fonts/roundtrip.ttf");
    let opentype_path = AssetPath::parse("fonts/roundtrip.otf");
    let truetype_bytes = b"\x00\x01\x00\x00roundtrip.ttf".to_vec();
    let opentype_bytes = b"OTTOroundtrip.otf".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(truetype_path.path(), truetype_bytes.clone());
    io.insert(opentype_path.path(), opentype_bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let truetype_id = database.import_asset_path(&truetype_path).unwrap();
    let opentype_id = database.import_asset_path(&opentype_path).unwrap();
    let truetype_metadata = database.registry().get(truetype_id).unwrap();
    let opentype_metadata = database.registry().get(opentype_id).unwrap();
    assert_eq!(truetype_metadata.asset_type, AssetTypeId::of::<Font>());
    assert_eq!(opentype_metadata.asset_type, AssetTypeId::of::<Font>());
    assert_eq!(truetype_metadata.importer.as_deref(), Some("FontImporter"));
    assert_eq!(opentype_metadata.importer.as_deref(), Some("FontImporter"));
    assert_eq!(truetype_metadata.importer_version, 3);
    assert_eq!(opentype_metadata.importer_version, 3);
    assert_eq!(
        fs::read(config.imported_root.join(truetype_path.path())).unwrap(),
        truetype_bytes
    );
    assert_eq!(
        fs::read(config.imported_root.join(opentype_path.path())).unwrap(),
        opentype_bytes
    );

    let truetype_output = database
        .cook_asset(truetype_id, TargetPlatform::Windows)
        .unwrap();
    let opentype_output = database
        .cook_asset(opentype_id, TargetPlatform::Windows)
        .unwrap();
    assert_eq!(truetype_output.bytes, truetype_bytes);
    assert_eq!(opentype_output.bytes, opentype_bytes);
    assert_eq!(
        fs::read(config.cooked_root.join(truetype_path.path())).unwrap(),
        truetype_bytes
    );
    assert_eq!(
        fs::read(config.cooked_root.join(opentype_path.path())).unwrap(),
        opentype_bytes
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let truetype: Handle<Font> = server.load(truetype_path);
    let opentype: Handle<Font> = server.load(opentype_path);
    server.update_loading();

    assert!(server.is_ready(&truetype));
    assert!(server.is_ready(&opentype));
    let truetype_loaded = server.get(&truetype).unwrap();
    let opentype_loaded = server.get(&opentype).unwrap();
    assert_eq!(truetype_loaded.family_name, "roundtrip");
    assert_eq!(opentype_loaded.family_name, "roundtrip");
    assert!(matches!(truetype_loaded.data, FontData::TrueType(ref bytes) if bytes == &truetype_bytes));
    assert!(matches!(opentype_loaded.data, FontData::OpenType(ref bytes) if bytes == &opentype_bytes));
}

#[test]
fn database_font_importer_reports_invalid_source_bitmap() {
    let config = database_config("font_importer_invalid_source");
    let path = AssetPath::parse("fonts/invalid.font");
    let mut io = MemoryAssetIo::new();
    io.insert(
        path.path(),
        b"NGA_FONT_SOURCE_V1\nfamily=Broken\nglyph=char=A;size=2x1;bitmap=1\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `FontImporter` failed")
                && message.contains("font source glyph bitmap on line 3 has 1 bytes, expected 2")
                && message.contains("fonts/invalid.font")
    ));
}

#[test]
fn database_font_importer_reports_invalid_binary_font() {
    let config = database_config("font_importer_invalid_binary_font");
    let path = AssetPath::parse("fonts/broken.otf");
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), b"not a real font".to_vec());
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `FontImporter` failed")
                && message.contains("OpenType font source has unsupported signature")
                && message.contains("fonts/broken.otf")
    ));
}

#[test]
fn database_builtin_physics_mesh_import_cook_and_runtime_load_preserves_payload() {
    let config = database_config("builtin_physics_mesh_runtime_load");
    let path = AssetPath::parse("physics/hero.physics");
    let bytes = physics_mesh_bytes();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), bytes.clone());
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<PhysicsMesh>());
    assert_eq!(metadata.importer.as_deref(), Some("PhysicsMeshImporter"));
    assert_eq!(metadata.importer_version, 2);
    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();

    assert_eq!(output.bytes, bytes);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        bytes
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let mesh: Handle<PhysicsMesh> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&mesh));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&mesh).unwrap();
    assert_eq!(loaded.kind, PhysicsMeshKind::TriMesh);
    assert_eq!(loaded.vertices.len(), 3);
    assert_eq!(loaded.indices, vec![[0, 1, 2]]);
}

#[test]
fn database_physics_mesh_importer_canonicalizes_source_to_runtime_bytes() {
    let config = database_config("physics_mesh_importer_source_conversion");
    let path = AssetPath::parse("physics/generated.physics");
    let source = b"NGA_PHYSICS_MESH_SOURCE_V1\n# source accepts key/value and directive forms\nkind = tri_mesh\nvertex = 0.0, 0.0, 0.0\nv 1.50 0 0\nv 0 1 0\ntriangle = 0, 1, 2\n".to_vec();
    let expected =
        b"NGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1.5 0 0\nv 0 1 0\ni 0 1 2\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(path.path(), source);
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let id = database.import_asset_path(&path).unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.asset_type, AssetTypeId::of::<PhysicsMesh>());
    assert_eq!(metadata.importer.as_deref(), Some("PhysicsMeshImporter"));
    assert_eq!(metadata.importer_version, 2);
    assert_eq!(metadata.cooked_path.as_ref(), Some(&path));
    assert_eq!(
        fs::read(config.imported_root.join(path.path())).unwrap(),
        expected
    );

    let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
    assert_eq!(output.bytes, expected);
    assert_eq!(
        fs::read(config.cooked_root.join(path.path())).unwrap(),
        expected
    );
    database.save_registry().unwrap();
    database.save_all_metadata_sidecars().unwrap();
    let mut loaded_sidecars = AssetDatabase::new(config.clone());
    loaded_sidecars.load_metadata_sidecars().unwrap();
    assert_eq!(
        loaded_sidecars
            .registry()
            .get(id)
            .unwrap()
            .cooked_path
            .as_ref(),
        Some(&path)
    );

    let mut server = AssetServer::new(AssetServerConfig {
        root: config.cooked_root.clone(),
        ..AssetServerConfig::default()
    });
    server.register_builtin_loaders();
    let mesh: Handle<PhysicsMesh> = server.load(path);
    server.update_loading();

    assert!(server.is_ready(&mesh));
    let loaded = server.get(&mesh).unwrap();
    assert_eq!(loaded.kind, PhysicsMeshKind::TriMesh);
    assert_eq!(
        loaded.vertices,
        vec![[0.0, 0.0, 0.0], [1.5, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(loaded.indices, vec![[0, 1, 2]]);
}

#[test]
fn database_physics_mesh_importer_reports_invalid_source_index() {
    let config = database_config("physics_mesh_importer_invalid_source");
    let path = AssetPath::parse("physics/invalid.physics");
    let mut io = MemoryAssetIo::new();
    io.insert(
        path.path(),
        b"NGA_PHYSICS_MESH_SOURCE_V1\nkind=trimesh\nv 0 0 0\ni 0 1 2\n".to_vec(),
    );
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();

    let error = database.import_asset_path(&path).unwrap_err();

    assert!(matches!(
        error,
        AssetError::Import { message }
            if message.contains("importer `PhysicsMeshImporter` failed")
                && message.contains("physics mesh source index 1 references missing vertex; vertex count is 1")
                && message.contains("physics/invalid.physics")
    ));
}

#[test]
#[cfg(feature = "bundle")]
fn database_builds_bundle_from_cooked_assets_and_preserves_dependencies() {
    let config = database_config("bundle_build");
    let texture_path = AssetPath::parse("textures/albedo.texture");
    let material_path = AssetPath::parse("materials/hero.material");
    let texture_source = texture_bytes(1, 1, 77);
    let material_source = b"name=hero\ntexture.albedo=textures/albedo.texture\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(texture_path.path(), texture_source.clone());
    io.insert(material_path.path(), material_source.clone());

    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    let material_id = database.import_asset_path(&material_path).unwrap();
    database
        .cook_asset(texture_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();

    let output = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "level_01",
            vec![material_id, texture_id],
        ))
        .unwrap();
    assert_eq!(output.asset_count, 2);

    let reader = BundleReader::from_bytes(&output.bytes).unwrap();
    assert_eq!(
        reader.manifest().dependencies(material_id),
        Some([texture_id].as_slice())
    );
    assert_eq!(reader.read_path(&texture_path).unwrap(), texture_source);
    assert_eq!(
        reader.read_path(&material_path).unwrap(),
        material_source.clone()
    );

    let bundle_io = BundleAssetIo::from_bytes(&output.bytes).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&output.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..6 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(material_id), AssetLoadState::Ready);
}

#[test]
#[cfg(feature = "bundle")]
fn database_builds_rle_compressed_bundle_and_runtime_preloads_it() {
    let config = database_config("bundle_build_rle");
    let texture_path = AssetPath::parse("textures/compressed.texture");
    let texture_b_path = AssetPath::parse("textures/compressed_b.texture");
    let texture_source = texture_bytes(8, 8, 66);
    let texture_b_source = texture_bytes(8, 8, 77);
    let mut io = MemoryAssetIo::new();
    io.insert(texture_path.path(), texture_source.clone());
    io.insert(texture_b_path.path(), texture_b_source.clone());

    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    let texture_b_id = database.import_asset_path(&texture_b_path).unwrap();
    database
        .cook_asset(texture_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(texture_b_id, TargetPlatform::Windows)
        .unwrap();

    let output = database
        .build_bundle(
            &AssetDatabaseBundleBuild::new("compressed_level", vec![texture_id, texture_b_id])
                .with_compression(CompressionKind::Rle)
                .with_chunk_policy(BundleChunkPartitionPolicy::MaxUncompressedBytes(
                    texture_source.len() + 1,
                )),
        )
        .unwrap();
    assert_eq!(output.asset_count, 2);

    let reader = BundleReader::from_bytes_with_loading_policy(
        &output.bytes,
        BundleChunkLoadingPolicy::OnDemandCached,
    )
    .unwrap();
    let chunk = &reader.manifest().chunks[0];
    assert_eq!(reader.manifest().compression, CompressionKind::Rle);
    assert_eq!(reader.manifest().chunks.len(), 2);
    assert_eq!(chunk.compression, CompressionKind::Rle);
    assert!(chunk.compressed_length < chunk.uncompressed_length);
    assert_eq!(reader.read_path(&texture_path).unwrap(), texture_source);
    let (range, report) = reader
        .read_path_range_with_report(&texture_path, 8, 16)
        .unwrap();
    assert_eq!(range, texture_source[8..24]);
    assert_eq!(report.entry, texture_id);
    assert_eq!(report.chunk_compression, CompressionKind::Rle);
    assert_eq!(report.cache_status, BundleChunkCacheStatus::Hit);
    assert_eq!(
        reader
            .read_path_with_report(&texture_b_path)
            .unwrap()
            .1
            .cache_status,
        BundleChunkCacheStatus::Miss
    );

    let bundle_io = BundleAssetIo::from_bytes_with_loading_policy(
        &output.bytes,
        BundleChunkLoadingPolicy::OnDemandCached,
    )
    .unwrap();
    assert_eq!(
        bundle_io.read_range(texture_path.path(), 8, 16).unwrap(),
        texture_source[8..24]
    );
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io.clone());
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&output.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..4 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(texture_b_id), AssetLoadState::Ready);
    assert_eq!(bundle_io.chunk_cache_stats().decoded_chunks, 2);
}

#[cfg(feature = "zstd")]
#[test]
#[cfg(feature = "bundle")]
fn database_builds_zstd_compressed_bundle_and_runtime_preloads_it() {
    let config = database_config("bundle_build_zstd");
    let texture_path = AssetPath::parse("textures/zstd.texture");
    let texture_b_path = AssetPath::parse("textures/zstd_b.texture");
    let texture_source = texture_bytes(16, 16, 88);
    let texture_b_source = texture_bytes(16, 16, 99);
    let mut io = MemoryAssetIo::new();
    io.insert(texture_path.path(), texture_source.clone());
    io.insert(texture_b_path.path(), texture_b_source.clone());

    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    let texture_b_id = database.import_asset_path(&texture_b_path).unwrap();
    database
        .cook_asset(texture_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(texture_b_id, TargetPlatform::Windows)
        .unwrap();

    let output = database
        .build_bundle(
            &AssetDatabaseBundleBuild::new("zstd_level", vec![texture_id, texture_b_id])
                .with_compression(CompressionKind::Zstd)
                .with_chunk_policy(BundleChunkPartitionPolicy::MaxUncompressedBytes(
                    texture_source.len() + 1,
                )),
        )
        .unwrap();
    assert_eq!(output.asset_count, 2);

    let reader = BundleReader::from_bytes_with_loading_policy(
        &output.bytes,
        BundleChunkLoadingPolicy::OnDemandCached,
    )
    .unwrap();
    assert_eq!(reader.manifest().compression, CompressionKind::Zstd);
    assert_eq!(reader.manifest().chunks.len(), 2);
    let chunk = &reader.manifest().chunks[0];
    assert_eq!(chunk.compression, CompressionKind::Zstd);
    assert!(chunk.compressed_length < chunk.uncompressed_length);
    let (range, report) = reader
        .read_path_range_with_report(&texture_path, 8, 32)
        .unwrap();
    assert_eq!(range, texture_source[8..40]);
    assert_eq!(report.entry, texture_id);
    assert_eq!(report.chunk_compression, CompressionKind::Zstd);
    assert_eq!(report.cache_status, BundleChunkCacheStatus::Miss);

    let bundle_io = BundleAssetIo::from_bytes_with_loading_policy(
        &output.bytes,
        BundleChunkLoadingPolicy::OnDemandCached,
    )
    .unwrap();
    assert_eq!(
        bundle_io.read_range(texture_b_path.path(), 8, 32).unwrap(),
        texture_b_source[8..40]
    );
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io.clone());
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&output.bytes).unwrap();
    let group = server.preload_bundle(&mounted);
    for _ in 0..4 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(texture_b_id), AssetLoadState::Ready);
    assert_eq!(bundle_io.chunk_cache_stats().decoded_chunks, 2);
}

#[test]
fn database_build_bundle_reports_missing_metadata_and_cooked_file_errors() {
    let config = database_config("bundle_build_errors");
    let mut database = AssetDatabase::new(config.clone());
    let missing_id = AssetId::new();
    assert!(matches!(
        database.build_bundle_bytes(&AssetDatabaseBundleBuild::new("missing", vec![missing_id])),
        Err(AssetError::AssetNotFound { id }) if id == missing_id
    ));

    let path = AssetPath::parse("textures/missing.texture");
    let id = AssetId::new();
    database.registry_mut().insert(AssetMetadata::runtime(
        id,
        path.clone(),
        AssetTypeId::of::<Texture>(),
    ));
    assert!(matches!(
        database.build_bundle_bytes(&AssetDatabaseBundleBuild::new("uncooked", vec![id])),
        Err(AssetError::Bundle { message }) if message.contains("no cooked path")
    ));

    let mut metadata = database.registry().get(id).unwrap().clone();
    metadata.cooked_path = Some(path.clone());
    metadata.cooked_hash = Some(ContentHash(0x1234));
    database.registry_mut().insert(metadata.clone());
    assert!(matches!(
        database.build_bundle_bytes(&AssetDatabaseBundleBuild::new("missing_file", vec![id])),
        Err(AssetError::Io { message }) if message.contains("failed to read cooked asset")
    ));

    let cooked_file = config.cooked_root.join(path.path());
    fs::create_dir_all(cooked_file.parent().unwrap()).unwrap();
    fs::write(&cooked_file, b"stale").unwrap();
    assert!(matches!(
        database.build_bundle_bytes(&AssetDatabaseBundleBuild::new("stale_hash", vec![id])),
        Err(AssetError::Bundle { message }) if message.contains("cooked hash mismatch")
    ));
}

#[test]
fn database_cook_reports_missing_metadata_and_missing_source_errors() {
    let config = database_config("cook_errors");
    let imported_payload = config.imported_root.join("textures/source.texture");
    let mut io = MemoryAssetIo::new();
    io.insert("textures/source.texture", texture_bytes(1, 1, 1));
    let mut database = AssetDatabase::new(config);
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    assert!(matches!(
        database.cook_asset(AssetId::new(), TargetPlatform::Windows),
        Err(AssetError::AssetNotFound { .. })
    ));

    let id = database
        .import_asset_path(&AssetPath::parse("textures/source.texture"))
        .unwrap();
    fs::remove_file(imported_payload).unwrap();
    database.set_io(MemoryAssetIo::new());

    assert!(matches!(
        database.cook_asset(id, TargetPlatform::Windows),
        Err(AssetError::Io { .. })
    ));
}

#[test]
fn database_registry_reports_header_and_line_context_for_malformed_text() {
    let config = database_config("registry_compat_errors");
    let mut database = AssetDatabase::new(config);

    assert!(matches!(
        database.load_registry_from_str("BAD_HEADER"),
        Err(AssetError::Io { message }) if message.contains("invalid asset registry header")
    ));
    assert!(matches!(
        database.load_registry_from_str("NGA_ASSET_REGISTRY_V1\n1|2"),
        Err(AssetError::Io { message })
            if message.contains("registry line 2") && message.contains("expected 12, 13, or 14 fields")
    ));
    assert!(matches!(
        database.load_registry_from_str("NGA_ASSET_REGISTRY_V0\n"),
        Err(AssetError::Io { message })
            if message.contains("unsupported asset registry version")
                && message.contains("run metadata migration")
    ));
}

#[test]
fn database_sidecar_reports_file_context_for_malformed_metadata() {
    let config = database_config("sidecar_compat_errors");
    let sidecar = config.imported_root.join("textures/bad.texture.meta");
    fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
    fs::write(&sidecar, "BAD_HEADER\n").unwrap();
    let mut database = AssetDatabase::new(config);

    assert!(matches!(
        database.load_metadata_sidecars(),
        Err(AssetError::Io { message })
            if message.contains("bad.texture.meta")
                && message.contains("invalid asset metadata sidecar header")
    ));
}

#[test]
fn database_loads_v1_compatible_metadata_without_labels() {
    let config = database_config("metadata_v1_compat_no_labels");
    let mut database = AssetDatabase::new(config);
    let id = AssetId::new();
    let path = AssetPath::parse("textures/no_labels.texture");
    let old_layout = [
        id.raw().to_string(),
        AssetTypeId::of::<Texture>().raw().to_string(),
        path.display_string(),
        path.display_string(),
        path.display_string(),
        "TextureImporter".to_owned(),
        "1".to_owned(),
        "11".to_owned(),
        "22".to_owned(),
        "33".to_owned(),
        "44".to_owned(),
        String::new(),
    ]
    .join("|");

    database
        .load_registry_from_str(&format!("NGA_ASSET_REGISTRY_V1\n{old_layout}"))
        .unwrap();
    let metadata = database.registry().get(id).unwrap();
    assert_eq!(metadata.path.as_ref(), Some(&path));
    assert_eq!(metadata.labels, Vec::<String>::new());
    assert_eq!(metadata.importer_settings, Vec::<(String, String)>::new());
    assert_eq!(metadata.source_hash, Some(ContentHash(11)));
    assert_eq!(metadata.version_hash, Some(VersionHash(44)));
}

#[test]
fn database_metadata_migration_report_classifies_registry_without_mutating_registry() {
    let config = database_config("metadata_migration_registry");
    fs::create_dir_all(config.registry_path.parent().unwrap()).unwrap();
    let old_id = AssetId::new();
    let current_id = AssetId::new();
    let old_path = AssetPath::parse("textures/old.texture");
    let current_path = AssetPath::parse("textures/current.texture");
    let registry_text = format!(
        "NGA_ASSET_REGISTRY_V1\n{}\n{}",
        metadata_line_with_fields(old_id, &old_path, 12),
        metadata_line_with_fields(current_id, &current_path, 14),
    );
    fs::write(&config.registry_path, registry_text).unwrap();
    let database = AssetDatabase::new(config.clone());

    let report = database.metadata_migration_report().unwrap();
    assert_eq!(database.registry().values().count(), 0);
    assert_eq!(report.total_entries(), 2);
    assert_eq!(report.upgradeable_entries(), 1);
    assert!(!report.has_blocking_errors());

    let registry = report
        .files
        .iter()
        .find(|file| file.kind == AssetMetadataMigrationFileKind::Registry)
        .unwrap();
    assert_eq!(registry.path, config.registry_path);
    assert_eq!(registry.header.as_deref(), Some("NGA_ASSET_REGISTRY_V1"));
    assert_eq!(registry.target_header, "NGA_ASSET_REGISTRY_V1");
    assert_eq!(registry.status, AssetMetadataMigrationStatus::Upgradeable);
    assert_eq!(registry.current_entries(), 1);
    assert_eq!(registry.upgradeable_entries(), 1);
    assert_eq!(registry.entries[0].id, Some(old_id));
    assert_eq!(registry.entries[0].field_count, 12);
    assert_eq!(
        registry.entries[0].status,
        AssetMetadataMigrationStatus::Upgradeable
    );
    assert_eq!(registry.entries[1].id, Some(current_id));
    assert_eq!(registry.entries[1].field_count, 14);
    assert_eq!(
        registry.entries[1].status,
        AssetMetadataMigrationStatus::Current
    );
}

#[test]
fn database_metadata_migration_upgrades_legacy_v0_registry_and_sidecar() {
    let config = database_config("metadata_migration_v0");
    fs::create_dir_all(config.registry_path.parent().unwrap()).unwrap();
    let registry_id = AssetId::new();
    let sidecar_id = AssetId::new();
    let dependency_id = AssetId::new();
    let registry_path = AssetPath::parse("models/hero.model#Mesh0");
    let registry_source = AssetPath::parse("models/hero.model");
    let registry_cooked = AssetPath::parse("models/hero.mesh");
    let sidecar_path = AssetPath::parse("textures/legacy.texture");
    let sidecar_source = AssetPath::parse("sources/legacy.png");
    let sidecar_cooked = AssetPath::parse("cooked/legacy.texture");
    let sidecar = config.imported_root.join("textures/legacy.texture.meta");
    fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
    let registry_text = format!(
        "NGA_ASSET_REGISTRY_V0\n{}",
        legacy_v0_metadata_line(
            registry_id,
            &registry_path,
            &registry_source,
            &registry_cooked,
            &[dependency_id],
        )
    );
    let sidecar_text = format!(
        "NGA_ASSET_META_V0\n{}",
        legacy_v0_metadata_line(
            sidecar_id,
            &sidecar_path,
            &sidecar_source,
            &sidecar_cooked,
            &[registry_id],
        )
    );
    fs::write(&config.registry_path, &registry_text).unwrap();
    fs::write(&sidecar, &sidecar_text).unwrap();
    let database = AssetDatabase::new(config.clone());

    let dry_run = database.metadata_migration_report().unwrap();
    assert_eq!(dry_run.total_entries(), 2);
    assert_eq!(dry_run.upgradeable_entries(), 2);
    assert!(!dry_run.has_blocking_errors());
    assert_eq!(dry_run.written_files(), 0);
    let registry_report = dry_run
        .files
        .iter()
        .find(|file| file.kind == AssetMetadataMigrationFileKind::Registry)
        .unwrap();
    assert_eq!(
        registry_report.header.as_deref(),
        Some("NGA_ASSET_REGISTRY_V0")
    );
    assert_eq!(
        registry_report.status,
        AssetMetadataMigrationStatus::Upgradeable
    );
    assert_eq!(registry_report.entries[0].field_count, 11);
    assert_eq!(registry_report.entries[0].id, Some(registry_id));
    let sidecar_report = dry_run
        .files
        .iter()
        .find(|file| file.kind == AssetMetadataMigrationFileKind::Sidecar)
        .unwrap();
    assert_eq!(sidecar_report.header.as_deref(), Some("NGA_ASSET_META_V0"));
    assert_eq!(
        sidecar_report.status,
        AssetMetadataMigrationStatus::Upgradeable
    );
    assert_eq!(sidecar_report.entries[0].field_count, 11);
    assert_eq!(sidecar_report.entries[0].id, Some(sidecar_id));

    let write = database
        .migrate_metadata(AssetMetadataMigrationMode::Write)
        .unwrap();
    assert_eq!(write.written_files(), 2);
    assert!(write
        .files
        .iter()
        .all(|file| { file.status != AssetMetadataMigrationStatus::Upgradeable || file.written }));
    assert!(fs::read_to_string(&config.registry_path)
        .unwrap()
        .starts_with("NGA_ASSET_REGISTRY_V1\n"));
    assert!(fs::read_to_string(&sidecar)
        .unwrap()
        .starts_with("NGA_ASSET_META_V1\n"));

    let mut loaded_registry = AssetDatabase::new(config.clone());
    loaded_registry.load_registry().unwrap();
    let registry_metadata = loaded_registry.registry().get(registry_id).unwrap();
    assert_eq!(registry_metadata.path.as_ref(), Some(&registry_path));
    assert_eq!(
        registry_metadata.source_path.as_ref(),
        Some(&registry_source)
    );
    assert_eq!(
        registry_metadata.cooked_path.as_ref(),
        Some(&registry_cooked)
    );
    assert_eq!(registry_metadata.dependencies, vec![dependency_id]);
    assert_eq!(registry_metadata.version_hash, None);
    assert_eq!(registry_metadata.labels, Vec::<String>::new());
    assert_eq!(
        registry_metadata.importer_settings,
        Vec::<(String, String)>::new()
    );

    let mut loaded_sidecar = AssetDatabase::new(config);
    loaded_sidecar.load_metadata_sidecars().unwrap();
    let sidecar_metadata = loaded_sidecar.registry().get(sidecar_id).unwrap();
    assert_eq!(sidecar_metadata.path.as_ref(), Some(&sidecar_path));
    assert_eq!(sidecar_metadata.source_path.as_ref(), Some(&sidecar_source));
    assert_eq!(sidecar_metadata.cooked_path.as_ref(), Some(&sidecar_cooked));
    assert_eq!(sidecar_metadata.dependencies, vec![registry_id]);
    assert_eq!(sidecar_metadata.version_hash, None);
}

#[test]
fn database_metadata_migration_report_surfaces_unsupported_and_invalid_sidecars() {
    let config = database_config("metadata_migration_sidecars");
    let unsupported = config
        .imported_root
        .join("textures/unsupported.texture.meta");
    let invalid = config.imported_root.join("textures/invalid.texture.meta");
    fs::create_dir_all(unsupported.parent().unwrap()).unwrap();
    fs::write(&unsupported, "NGA_ASSET_META_V9\n").unwrap();
    fs::write(&invalid, "NGA_ASSET_META_V1\n1|2").unwrap();
    let database = AssetDatabase::new(config);

    let report = database.metadata_migration_report().unwrap();
    assert!(report.has_blocking_errors());
    let sidecars = report
        .files
        .iter()
        .filter(|file| file.kind == AssetMetadataMigrationFileKind::Sidecar)
        .collect::<Vec<_>>();
    assert_eq!(sidecars.len(), 2);
    let unsupported_report = sidecars
        .iter()
        .find(|file| file.path.ends_with("unsupported.texture.meta"))
        .unwrap();
    assert_eq!(
        unsupported_report.status,
        AssetMetadataMigrationStatus::UnsupportedVersion
    );
    assert!(unsupported_report.errors[0].contains("unsupported metadata version"));

    let invalid_report = sidecars
        .iter()
        .find(|file| file.path.ends_with("invalid.texture.meta"))
        .unwrap();
    assert_eq!(invalid_report.status, AssetMetadataMigrationStatus::Invalid);
    assert_eq!(invalid_report.entries.len(), 1);
    assert_eq!(
        invalid_report.entries[0].status,
        AssetMetadataMigrationStatus::Invalid
    );
    assert!(invalid_report.errors[0].contains("expected 12, 13, or 14 fields"));
}

#[test]
fn database_metadata_migration_write_back_upgrades_registry_and_sidecars() {
    let config = database_config("metadata_migration_write_back");
    fs::create_dir_all(config.registry_path.parent().unwrap()).unwrap();
    let registry_id = AssetId::new();
    let sidecar_id = AssetId::new();
    let registry_path = AssetPath::parse("textures/registry_old.texture");
    let sidecar_asset_path = AssetPath::parse("textures/sidecar_old.texture");
    let sidecar = config
        .imported_root
        .join("textures/sidecar_old.texture.meta");
    fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
    let registry_text = format!(
        "NGA_ASSET_REGISTRY_V1\n{}",
        metadata_line_with_fields(registry_id, &registry_path, 12)
    );
    let sidecar_text = format!(
        "NGA_ASSET_META_V1\n{}",
        metadata_line_with_fields(sidecar_id, &sidecar_asset_path, 13)
    );
    fs::write(&config.registry_path, registry_text.clone()).unwrap();
    fs::write(&sidecar, sidecar_text.clone()).unwrap();
    let database = AssetDatabase::new(config.clone());

    let dry_run = database.metadata_migration_report().unwrap();
    assert_eq!(dry_run.mode, AssetMetadataMigrationMode::DryRun);
    assert_eq!(dry_run.upgradeable_entries(), 2);
    assert_eq!(dry_run.written_files(), 0);
    assert_eq!(
        fs::read_to_string(&config.registry_path).unwrap(),
        registry_text
    );
    assert_eq!(fs::read_to_string(&sidecar).unwrap(), sidecar_text);

    let write = database
        .migrate_metadata(AssetMetadataMigrationMode::Write)
        .unwrap();
    assert_eq!(write.mode, AssetMetadataMigrationMode::Write);
    assert_eq!(write.written_files(), 2);
    assert!(write
        .files
        .iter()
        .all(|file| { file.status != AssetMetadataMigrationStatus::Upgradeable || file.written }));

    let migrated_registry = fs::read_to_string(&config.registry_path).unwrap();
    let migrated_registry_payload = migrated_registry.lines().nth(1).unwrap();
    assert_eq!(migrated_registry_payload.split('|').count(), 14);
    assert!(migrated_registry_payload.ends_with('|'));
    let mut loaded_registry = AssetDatabase::new(config.clone());
    loaded_registry.load_registry().unwrap();
    assert_eq!(
        loaded_registry.registry().get(registry_id).unwrap().labels,
        Vec::<String>::new()
    );

    let migrated_sidecar = fs::read_to_string(&sidecar).unwrap();
    let migrated_sidecar_payload = migrated_sidecar.lines().nth(1).unwrap();
    assert_eq!(migrated_sidecar_payload.split('|').count(), 14);
    assert!(migrated_sidecar_payload.contains("|label|"));
    let mut loaded_sidecar = AssetDatabase::new(config.clone());
    loaded_sidecar.load_metadata_sidecars().unwrap();
    let metadata = loaded_sidecar.registry().get(sidecar_id).unwrap();
    assert_eq!(metadata.labels, vec!["label"]);
    assert_eq!(metadata.importer_settings, Vec::<(String, String)>::new());

    let after = database.metadata_migration_report().unwrap();
    assert_eq!(after.upgradeable_entries(), 0);
    assert!(after
        .files
        .iter()
        .all(|file| file.status == AssetMetadataMigrationStatus::Current));
}

#[test]
fn database_metadata_migration_write_back_leaves_invalid_and_unsupported_files_unchanged() {
    let config = database_config("metadata_migration_write_back_errors");
    let unsupported = config
        .imported_root
        .join("textures/unsupported.texture.meta");
    let invalid = config.imported_root.join("textures/invalid.texture.meta");
    let upgradeable = config
        .imported_root
        .join("textures/upgradeable.texture.meta");
    fs::create_dir_all(unsupported.parent().unwrap()).unwrap();
    let unsupported_text = "NGA_ASSET_META_V9\n".to_owned();
    let invalid_text = "NGA_ASSET_META_V1\n1|2".to_owned();
    let upgradeable_id = AssetId::new();
    let upgradeable_path = AssetPath::parse("textures/upgradeable.texture");
    let upgradeable_text = format!(
        "NGA_ASSET_META_V1\n{}",
        metadata_line_with_fields(upgradeable_id, &upgradeable_path, 12)
    );
    fs::write(&unsupported, &unsupported_text).unwrap();
    fs::write(&invalid, &invalid_text).unwrap();
    fs::write(&upgradeable, &upgradeable_text).unwrap();
    let database = AssetDatabase::new(config);

    let report = database
        .migrate_metadata(AssetMetadataMigrationMode::Write)
        .unwrap();
    assert!(report.has_blocking_errors());
    assert_eq!(report.written_files(), 1);
    assert_eq!(fs::read_to_string(&unsupported).unwrap(), unsupported_text);
    assert_eq!(fs::read_to_string(&invalid).unwrap(), invalid_text);
    assert_eq!(
        fs::read_to_string(&upgradeable)
            .unwrap()
            .lines()
            .nth(1)
            .unwrap()
            .split('|')
            .count(),
        14
    );
    let unsupported_report = report
        .files
        .iter()
        .find(|file| file.path == unsupported)
        .unwrap();
    assert_eq!(
        unsupported_report.status,
        AssetMetadataMigrationStatus::UnsupportedVersion
    );
    assert!(!unsupported_report.written);
    let invalid_report = report
        .files
        .iter()
        .find(|file| file.path == invalid)
        .unwrap();
    assert_eq!(invalid_report.status, AssetMetadataMigrationStatus::Invalid);
    assert!(!invalid_report.written);
}

#[test]
fn database_sidecar_reports_unsupported_version_with_file_context() {
    let config = database_config("sidecar_unsupported_version");
    let sidecar = config.imported_root.join("textures/old.texture.meta");
    fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
    fs::write(&sidecar, "NGA_ASSET_META_V0\n").unwrap();
    let mut database = AssetDatabase::new(config);

    assert!(matches!(
        database.load_metadata_sidecars(),
        Err(AssetError::Io { message })
            if message.contains("old.texture.meta")
                && message.contains("unsupported asset metadata sidecar version")
                && message.contains("run metadata migration")
    ));
}
