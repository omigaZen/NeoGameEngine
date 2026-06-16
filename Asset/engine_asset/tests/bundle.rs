use engine_asset::prelude::*;

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
}

fn scene_bytes(name: &str, dependency_path: &str) -> Vec<u8> {
    format!(
        "NGA_SCENE_V1\nname={name}\ndependency={dependency_path}\nentity=Root\ncomponent=Transform|translation=0,0,0\n"
    )
    .into_bytes()
}

fn prefab_bytes(name: &str, dependency_path: &str) -> Vec<u8> {
    format!(
        "NGA_PREFAB_V1\ndependency={dependency_path}\nroot={name}\ncomponent=Transform|translation=0,0,0\nchild={name}_child;parent=0\ncomponent=Transform|translation=1,0,0\n"
    )
    .into_bytes()
}

fn content_hash(bytes: &[u8]) -> ContentHash {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    ContentHash(hash)
}

fn texture_bundle(path: &str, bytes: Vec<u8>) -> (AssetId, Vec<u8>) {
    let id = AssetId::new();
    let bundle = BundleWriter::build_bytes(
        "textures",
        CompressionKind::None,
        vec![BundleAsset {
            id,
            asset_type: AssetTypeId::of::<Texture>(),
            path: AssetPath::parse(path),
            bytes,
            dependencies: Vec::new(),
        }],
    )
    .unwrap();
    (id, bundle)
}

fn texture_bundle_io(name: &str, files: Vec<(&str, Vec<u8>)>) -> BundleAssetIo {
    let assets = files
        .into_iter()
        .map(|(path, bytes)| BundleAsset {
            id: AssetId::new(),
            asset_type: AssetTypeId::of::<Texture>(),
            path: AssetPath::parse(path),
            bytes,
            dependencies: Vec::new(),
        })
        .collect::<Vec<_>>();
    let bundle = BundleWriter::build_bytes(name, CompressionKind::None, assets).unwrap();
    BundleAssetIo::from_bytes(&bundle).unwrap()
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

fn texture_package(
    name: &str,
    kind: AssetIoLayerKind,
    priority: usize,
    bundle_id: BundleId,
    bundle_path: &str,
    files: Vec<(&str, Vec<u8>)>,
) -> (AssetPackageRecord, Vec<u8>, Vec<AssetId>) {
    let mut ids = Vec::new();
    let assets = files
        .into_iter()
        .map(|(path, bytes)| {
            let id = AssetId::new();
            ids.push(id);
            BundleAsset {
                id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse(path),
                bytes,
                dependencies: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    let bundle = BundleWriter::build_bytes(name, CompressionKind::None, assets).unwrap();
    let manifest = BundleReader::from_bytes(&bundle)
        .unwrap()
        .manifest()
        .clone();
    (
        AssetPackageRecord::new(bundle_id, name, kind, priority, true, bundle_path, manifest),
        bundle,
        ids,
    )
}

fn package_from_assets(
    name: &str,
    kind: AssetIoLayerKind,
    priority: usize,
    bundle_id: BundleId,
    bundle_path: &str,
    assets: Vec<BundleAsset>,
) -> (AssetPackageRecord, Vec<u8>) {
    let bundle = BundleWriter::build_bytes(name, CompressionKind::None, assets).unwrap();
    let manifest = BundleReader::from_bytes(&bundle)
        .unwrap()
        .manifest()
        .clone();
    (
        AssetPackageRecord::new(bundle_id, name, kind, priority, true, bundle_path, manifest),
        bundle,
    )
}

fn temp_file(name: &str, extension: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "engine_asset_{name}_{}.{}",
        AssetId::new().raw(),
        extension
    ))
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("engine_asset_{name}_{}", AssetId::new().raw()))
}

#[test]
fn bundle_writer_reader_round_trip_preserves_manifest_dependencies() {
    let shader_id = AssetId::new();
    let texture_id = AssetId::new();
    let bytes = texture_bytes(1, 1, 44);
    let bundle = BundleWriter::build_bytes(
        "level_01",
        CompressionKind::None,
        vec![BundleAsset {
            id: texture_id,
            asset_type: AssetTypeId::of::<Texture>(),
            path: AssetPath::parse("textures/albedo.texture"),
            bytes: bytes.clone(),
            dependencies: vec![shader_id],
        }],
    )
    .unwrap();

    let reader = BundleReader::from_bytes(&bundle).unwrap();
    assert_eq!(reader.manifest().name, "level_01");
    assert_eq!(
        reader.manifest().dependencies(texture_id),
        Some([shader_id].as_slice())
    );
    assert_eq!(
        reader
            .read_path(&AssetPath::parse("textures/albedo.texture"))
            .unwrap(),
        bytes
    );
    assert_eq!(reader.read_entry(texture_id).unwrap(), bytes);
}

#[test]
fn bundle_manifest_exposes_v2_chunk_layout_metadata() {
    let first_id = AssetId::new();
    let second_id = AssetId::new();
    let first = texture_bytes(1, 1, 11);
    let second = texture_bytes(2, 1, 22);
    let mut expected_data = Vec::new();
    expected_data.extend_from_slice(&first);
    expected_data.extend_from_slice(&second);

    let bundle = BundleWriter::build_bytes(
        "chunked_textures",
        CompressionKind::None,
        vec![
            BundleAsset {
                id: first_id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/first.texture"),
                bytes: first.clone(),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: second_id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/second.texture"),
                bytes: second.clone(),
                dependencies: vec![first_id],
            },
        ],
    )
    .unwrap();
    let marker = b"\nDATA\n";
    let marker_index = bundle
        .windows(marker.len())
        .position(|window| window == marker)
        .unwrap();
    let manifest_text = std::str::from_utf8(&bundle[..marker_index]).unwrap();
    assert!(manifest_text.starts_with("NGA_BUNDLE_V2"));
    assert!(manifest_text.contains("chunks=1"));

    let reader = BundleReader::from_bytes(&bundle).unwrap();
    let manifest = reader.manifest();
    assert_eq!(
        manifest.total_uncompressed_bytes(),
        expected_data.len() as u64
    );
    assert_eq!(manifest.chunks.len(), 1);
    assert_eq!(
        manifest.chunks[0],
        BundleChunk {
            index: 0,
            offset: 0,
            compressed_length: expected_data.len() as u64,
            uncompressed_length: expected_data.len() as u64,
            compression: CompressionKind::None,
            content_hash: content_hash(&expected_data),
        }
    );
    let first_entry = manifest.entry(first_id).unwrap();
    assert_eq!(first_entry.chunk_index, 0);
    assert_eq!(first_entry.offset, 0);
    assert_eq!(first_entry.length, first.len() as u64);
    let second_entry = manifest.entry(second_id).unwrap();
    assert_eq!(second_entry.chunk_index, 0);
    assert_eq!(second_entry.offset, first.len() as u64);
    assert_eq!(second_entry.length, second.len() as u64);
    assert_eq!(
        reader
            .read_path(&AssetPath::parse("textures/first.texture"))
            .unwrap(),
        first
    );
    assert_eq!(
        reader
            .read_path(&AssetPath::parse("textures/second.texture"))
            .unwrap(),
        second
    );
}

#[test]
fn bundle_reader_accepts_legacy_v1_manifest_as_single_uncompressed_chunk() {
    let id = AssetId::new();
    let payload = b"legacy payload".to_vec();
    let header = format!(
        "NGA_BUNDLE_V1\nname=legacy\ncompression=none\nentries=1\nentry|{}|{}|legacy/payload.bin|0|{}|{}|\nDATA\n",
        id.raw(),
        AssetTypeId::of::<Texture>().raw(),
        payload.len(),
        content_hash(&payload).0,
    );
    let mut bundle = header.into_bytes();
    bundle.extend_from_slice(&payload);

    let reader = BundleReader::from_bytes(&bundle).unwrap();
    assert_eq!(reader.manifest().name, "legacy");
    assert_eq!(reader.manifest().chunks.len(), 1);
    assert_eq!(
        reader.manifest().chunks[0],
        BundleChunk {
            index: 0,
            offset: 0,
            compressed_length: payload.len() as u64,
            uncompressed_length: payload.len() as u64,
            compression: CompressionKind::None,
            content_hash: content_hash(&payload),
        }
    );
    assert_eq!(reader.manifest().entry(id).unwrap().chunk_index, 0);
    assert_eq!(
        reader
            .read_path(&AssetPath::parse("legacy/payload.bin"))
            .unwrap(),
        payload
    );
}

#[test]
fn bundle_compression_codec_report_and_zstd_feature_diagnostics() {
    assert_eq!(
        BundleCompressionCodecReport::for_compression(CompressionKind::None),
        BundleCompressionCodecReport {
            compression: CompressionKind::None,
            supported: true,
            codec_name: "none",
            reason: None,
        }
    );
    assert_eq!(
        BundleCompressionCodecReport::for_compression(CompressionKind::Rle),
        BundleCompressionCodecReport {
            compression: CompressionKind::Rle,
            supported: true,
            codec_name: "rle",
            reason: None,
        }
    );
    let zstd = BundleCompressionCodecReport::for_compression(CompressionKind::Zstd);
    assert_eq!(zstd.codec_name, "zstd");
    assert_eq!(zstd.supported, asset_feature_enabled(AssetFeature::Zstd));

    #[cfg(feature = "zstd")]
    {
        assert!(zstd.reason.is_none());
        let first_id = AssetId::new();
        let second_id = AssetId::new();
        let first = vec![7_u8; 4096];
        let second = vec![13_u8; 2048];
        let bundle = BundleWriter::build_bytes_with_options(
            "zstd_textures",
            BundleBuildOptions::new(CompressionKind::Zstd).with_chunk_policy(
                BundleChunkPartitionPolicy::MaxUncompressedBytes(first.len()),
            ),
            vec![
                BundleAsset {
                    id: first_id,
                    asset_type: AssetTypeId::of::<Texture>(),
                    path: AssetPath::parse("textures/zstd_a.texture"),
                    bytes: first.clone(),
                    dependencies: Vec::new(),
                },
                BundleAsset {
                    id: second_id,
                    asset_type: AssetTypeId::of::<Texture>(),
                    path: AssetPath::parse("textures/zstd_b.texture"),
                    bytes: second.clone(),
                    dependencies: vec![first_id],
                },
            ],
        )
        .unwrap();
        let reader = BundleReader::from_bytes_with_loading_policy(
            &bundle,
            BundleChunkLoadingPolicy::OnDemandCached,
        )
        .unwrap();
        assert_eq!(reader.manifest().compression, CompressionKind::Zstd);
        assert_eq!(reader.manifest().chunks.len(), 2);
        let first_chunk = reader.manifest().chunk(0).unwrap();
        assert_eq!(first_chunk.compression, CompressionKind::Zstd);
        assert!(first_chunk.compressed_length < first_chunk.uncompressed_length);
        assert_eq!(reader.chunk_cache_stats().decoded_chunks, 0);

        let (range, report) = reader
            .read_path_range_with_report(&AssetPath::parse("textures/zstd_b.texture"), 512, 64)
            .unwrap();
        assert_eq!(range, second[512..576]);
        assert_eq!(report.entry, second_id);
        assert_eq!(report.chunk_index, 1);
        assert_eq!(report.chunk_compression, CompressionKind::Zstd);
        assert_eq!(report.cache_status, BundleChunkCacheStatus::Miss);
        assert_eq!(reader.chunk_cache_stats().cache_misses, 1);

        let bundle_io = BundleAssetIo::from_bytes_with_loading_policy(
            &bundle,
            BundleChunkLoadingPolicy::OnDemandCached,
        )
        .unwrap();
        assert_eq!(
            bundle_io
                .read_range("textures/zstd_a.texture", 4000, 128)
                .unwrap(),
            first[4000..4096]
        );

        let corrupted = b"NGA_BUNDLE_V2\nname=bad_zstd\ncompression=zstd\nchunks=1\nchunk|0|0|8|4|zstd|1\nentries=0\nDATA\nnotzstd!";
        assert!(matches!(
            BundleReader::from_bytes(corrupted),
            Err(AssetError::Bundle { message })
                if message.contains("zstd bundle chunk 0")
                    && message.contains("decode")
        ));
    }

    #[cfg(not(feature = "zstd"))]
    {
        assert!(zstd.reason.as_deref().unwrap().contains("zstd feature"));
        let bundle = b"NGA_BUNDLE_V2\nname=compressed\ncompression=none\nchunks=1\nchunk|3|0|4|4|zstd|1\nentries=0\nDATA\nabcd";
        assert!(matches!(
            BundleReader::from_bytes(bundle),
            Err(AssetError::Bundle { message })
                if message.contains("chunk 3")
                    && message.contains("codec `zstd`")
                    && message.contains("disabled")
        ));
    }
}

#[test]
fn bundle_rle_compression_round_trip_exposes_chunk_reports_and_ranges() {
    let first_id = AssetId::new();
    let second_id = AssetId::new();
    let first = texture_bytes(8, 8, 55);
    let second = vec![9_u8; 300];
    let mut expected_data = Vec::new();
    expected_data.extend_from_slice(&first);
    expected_data.extend_from_slice(&second);

    let bundle = BundleWriter::build_bytes(
        "rle_textures",
        CompressionKind::Rle,
        vec![
            BundleAsset {
                id: first_id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/rle.texture"),
                bytes: first.clone(),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: second_id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/repeated.texture"),
                bytes: second.clone(),
                dependencies: vec![first_id],
            },
        ],
    )
    .unwrap();

    let reader = BundleReader::from_bytes(&bundle).unwrap();
    let chunk = &reader.manifest().chunks[0];
    assert_eq!(reader.manifest().compression, CompressionKind::Rle);
    assert_eq!(chunk.compression, CompressionKind::Rle);
    assert_eq!(chunk.uncompressed_length, expected_data.len() as u64);
    assert!(chunk.compressed_length < chunk.uncompressed_length);
    assert_eq!(chunk.content_hash, content_hash(&expected_data));
    assert_eq!(
        reader
            .read_path(&AssetPath::parse("textures/rle.texture"))
            .unwrap(),
        first
    );
    assert_eq!(
        reader.read_entry_range(second_id, 250, 80).unwrap(),
        second[250..300]
    );

    let (range, report) = reader
        .read_path_range_with_report(&AssetPath::parse("textures/repeated.texture"), 10, 24)
        .unwrap();
    assert_eq!(range, second[10..34]);
    assert_eq!(report.entry, second_id);
    assert_eq!(
        report.path,
        Some(AssetPath::parse("textures/repeated.texture"))
    );
    assert_eq!(report.chunk_compression, CompressionKind::Rle);
    assert_eq!(report.chunk_compressed_length, chunk.compressed_length);
    assert_eq!(report.chunk_uncompressed_length, chunk.uncompressed_length);
    assert_eq!(report.range_offset, 10);
    assert_eq!(report.range_length, 24);
    assert_eq!(report.bytes_returned, 24);

    let bundle_io = BundleAssetIo::from_bytes(&bundle).unwrap();
    assert_eq!(
        bundle_io
            .read_range("textures/repeated.texture", 296, 20)
            .unwrap(),
        second[296..300]
    );
    assert_eq!(
        bundle_io
            .metadata("textures/repeated.texture")
            .unwrap()
            .hash,
        Some(content_hash(&second))
    );
}

#[test]
#[cfg(feature = "zstd")]
fn bundle_zstd_compression_round_trip_exposes_chunk_reports_and_prefetches() {
    let first_id = AssetId::new();
    let second_id = AssetId::new();
    let third_id = AssetId::new();
    let first = texture_bytes(1, 1, 21);
    let second = texture_bytes(1, 1, 22);
    let third = texture_bytes(1, 1, 23);
    let bundle = BundleWriter::build_bytes_with_options(
        "zstd_cache_textures",
        BundleBuildOptions::new(CompressionKind::Zstd).with_chunk_policy(
            BundleChunkPartitionPolicy::MaxUncompressedBytes(first.len()),
        ),
        vec![
            BundleAsset {
                id: first_id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/first.texture"),
                bytes: first.clone(),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: second_id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/second.texture"),
                bytes: second.clone(),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: third_id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/third.texture"),
                bytes: third.clone(),
                dependencies: Vec::new(),
            },
        ],
    )
    .unwrap();

    let reader = BundleReader::from_bytes_with_loading_policy(
        &bundle,
        BundleChunkLoadingPolicy::OnDemandCachedLimited {
            max_decoded_chunks: 1,
        },
    )
    .unwrap();
    assert_eq!(reader.manifest().compression, CompressionKind::Zstd);
    assert_eq!(reader.manifest().chunks.len(), 3);
    assert_eq!(reader.chunk_cache_stats().max_decoded_chunks, Some(1));

    let first_prefetch = reader
        .prefetch_path(&AssetPath::parse("textures/first.texture"))
        .unwrap();
    assert_eq!(first_prefetch.decoded_chunks, vec![0]);
    assert!(first_prefetch.evicted_chunks.is_empty());
    assert_eq!(reader.chunk_cache_stats().decoded_chunks, 1);
    assert_eq!(reader.chunk_cache_stats().prefetched_chunks, 1);

    let second_prefetch = reader.prefetch_chunk(1).unwrap();
    assert_eq!(second_prefetch.decoded_chunks, vec![1]);
    assert_eq!(second_prefetch.evicted_chunks, vec![0]);
    let stats = reader.chunk_cache_stats();
    assert_eq!(stats.decoded_chunks, 1);
    assert_eq!(stats.cache_misses, 2);
    assert_eq!(stats.cache_evictions, 1);
    assert_eq!(stats.prefetched_chunks, 2);

    let bundle_io = BundleAssetIo::from_bytes_with_loading_policy(
        &bundle,
        BundleChunkLoadingPolicy::OnDemandCachedLimited {
            max_decoded_chunks: 2,
        },
    )
    .unwrap();
    let prefetch = bundle_io
        .prefetch_paths(&["textures/first.texture", "textures/second.texture"])
        .unwrap();
    assert_eq!(prefetch.cache_misses, 2);
    assert_eq!(prefetch.decoded_chunks, vec![0, 1]);
    assert!(prefetch.evicted_chunks.is_empty());
    let third_prefetch = bundle_io.prefetch_path("textures/third.texture").unwrap();
    assert_eq!(third_prefetch.decoded_chunks, vec![2]);
    assert_eq!(third_prefetch.evicted_chunks, vec![0]);
    let stats = bundle_io.chunk_cache_stats();
    assert_eq!(stats.decoded_chunks, 2);
    assert_eq!(stats.cache_evictions, 1);
    assert_eq!(stats.prefetched_chunks, 3);

    let (range, report) = bundle_io
        .read_range_with_report("textures/second.texture", 8, 4)
        .unwrap();
    assert_eq!(range, second[8..12]);
    assert_eq!(report.entry, second_id);
    assert_eq!(report.chunk_index, 1);
    assert_eq!(report.chunk_compression, CompressionKind::Zstd);
    assert_eq!(report.cache_status, BundleChunkCacheStatus::Hit);
    assert_eq!(bundle_io.chunk_cache_stats().cache_hits, 1);
    assert_eq!(
        bundle_io
            .read_range("textures/first.texture", 0, 8)
            .unwrap(),
        first[0..8]
    );
    assert_eq!(
        bundle_io.metadata("textures/third.texture").unwrap().hash,
        Some(content_hash(&third))
    );
}

#[test]
fn bundle_chunk_partition_policy_and_on_demand_cache_are_observable() {
    let first_id = AssetId::new();
    let second_id = AssetId::new();
    let third_id = AssetId::new();
    let first = texture_bytes(1, 1, 1);
    let second = texture_bytes(2, 1, 2);
    let third = texture_bytes(1, 1, 3);
    assert!(matches!(
        BundleWriter::build_bytes_with_options(
            "bad_policy",
            BundleBuildOptions::new(CompressionKind::None)
                .with_chunk_policy(BundleChunkPartitionPolicy::MaxUncompressedBytes(0)),
            Vec::new(),
        ),
        Err(AssetError::Bundle { message }) if message.contains("max chunk size")
    ));
    let bundle = BundleWriter::build_bytes_with_options(
        "partitioned_textures",
        BundleBuildOptions::new(CompressionKind::Rle).with_chunk_policy(
            BundleChunkPartitionPolicy::MaxUncompressedBytes(first.len() + 1),
        ),
        vec![
            BundleAsset {
                id: first_id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/first.texture"),
                bytes: first.clone(),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: second_id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/second.texture"),
                bytes: second.clone(),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: third_id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/third.texture"),
                bytes: third.clone(),
                dependencies: Vec::new(),
            },
        ],
    )
    .unwrap();

    let reader = BundleReader::from_bytes_with_loading_policy(
        &bundle,
        BundleChunkLoadingPolicy::OnDemandCached,
    )
    .unwrap();
    assert_eq!(reader.manifest().chunks.len(), 3);
    assert_eq!(reader.manifest().entry(first_id).unwrap().chunk_index, 0);
    assert_eq!(reader.manifest().entry(second_id).unwrap().chunk_index, 1);
    assert_eq!(reader.manifest().entry(third_id).unwrap().chunk_index, 2);
    assert_eq!(
        reader.chunk_cache_stats(),
        BundleChunkCacheStats {
            policy: BundleChunkLoadingPolicy::OnDemandCached,
            chunks_total: 3,
            max_decoded_chunks: None,
            decoded_chunks: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_evictions: 0,
            prefetched_chunks: 0,
            decoded_bytes: 0,
        }
    );

    let (bytes, first_report) = reader
        .read_path_with_report(&AssetPath::parse("textures/first.texture"))
        .unwrap();
    assert_eq!(bytes, first);
    assert_eq!(first_report.chunk_index, 0);
    assert_eq!(first_report.cache_status, BundleChunkCacheStatus::Miss);
    assert_eq!(reader.chunk_cache_stats().decoded_chunks, 1);
    assert_eq!(reader.chunk_cache_stats().cache_misses, 1);

    let (_, first_again_report) = reader
        .read_path_with_report(&AssetPath::parse("textures/first.texture"))
        .unwrap();
    assert_eq!(first_again_report.cache_status, BundleChunkCacheStatus::Hit);
    assert_eq!(reader.chunk_cache_stats().cache_hits, 1);

    let (second_range, second_report) = reader
        .read_path_range_with_report(&AssetPath::parse("textures/second.texture"), 8, 4)
        .unwrap();
    assert_eq!(second_range, second[8..12]);
    assert_eq!(second_report.chunk_index, 1);
    assert_eq!(second_report.cache_status, BundleChunkCacheStatus::Miss);
    assert_eq!(reader.chunk_cache_stats().decoded_chunks, 2);

    let bundle_io = BundleAssetIo::from_bytes_with_loading_policy(
        &bundle,
        BundleChunkLoadingPolicy::OnDemandCached,
    )
    .unwrap();
    let (_, io_report) = bundle_io
        .read_range_with_report("textures/third.texture", 8, 4)
        .unwrap();
    assert_eq!(io_report.chunk_index, 2);
    assert_eq!(io_report.cache_status, BundleChunkCacheStatus::Miss);
    assert_eq!(bundle_io.chunk_cache_stats().decoded_chunks, 1);
}

#[test]
fn bundle_limited_chunk_cache_prefetches_and_evicts_lru_chunks() {
    let first = texture_bytes(1, 1, 11);
    let second = texture_bytes(1, 1, 12);
    let third = texture_bytes(1, 1, 13);
    let bundle = BundleWriter::build_bytes_with_options(
        "limited_cache_textures",
        BundleBuildOptions::new(CompressionKind::Rle).with_chunk_policy(
            BundleChunkPartitionPolicy::MaxUncompressedBytes(first.len()),
        ),
        vec![
            BundleAsset {
                id: AssetId::new(),
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/first.texture"),
                bytes: first.clone(),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: AssetId::new(),
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/second.texture"),
                bytes: second.clone(),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: AssetId::new(),
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/third.texture"),
                bytes: third.clone(),
                dependencies: Vec::new(),
            },
        ],
    )
    .unwrap();

    assert!(matches!(
        BundleReader::from_bytes_with_loading_policy(
            &bundle,
            BundleChunkLoadingPolicy::OnDemandCachedLimited {
                max_decoded_chunks: 0,
            },
        ),
        Err(AssetError::Bundle { message }) if message.contains("max decoded chunks")
    ));

    let reader = BundleReader::from_bytes_with_loading_policy(
        &bundle,
        BundleChunkLoadingPolicy::OnDemandCachedLimited {
            max_decoded_chunks: 1,
        },
    )
    .unwrap();
    assert_eq!(reader.chunk_cache_stats().max_decoded_chunks, Some(1));

    let first_prefetch = reader
        .prefetch_path(&AssetPath::parse("textures/first.texture"))
        .unwrap();
    assert_eq!(first_prefetch.decoded_chunks, vec![0]);
    assert!(first_prefetch.evicted_chunks.is_empty());
    assert_eq!(reader.chunk_cache_stats().decoded_chunks, 1);
    assert_eq!(reader.chunk_cache_stats().prefetched_chunks, 1);

    let second_prefetch = reader.prefetch_chunk(1).unwrap();
    assert_eq!(second_prefetch.decoded_chunks, vec![1]);
    assert_eq!(second_prefetch.evicted_chunks, vec![0]);
    let stats = reader.chunk_cache_stats();
    assert_eq!(stats.decoded_chunks, 1);
    assert_eq!(stats.cache_misses, 2);
    assert_eq!(stats.cache_evictions, 1);
    assert_eq!(stats.prefetched_chunks, 2);

    let (_, first_report) = reader
        .read_path_with_report(&AssetPath::parse("textures/first.texture"))
        .unwrap();
    assert_eq!(first_report.cache_status, BundleChunkCacheStatus::Miss);
    assert_eq!(reader.chunk_cache_stats().cache_evictions, 2);
    let (_, first_again_report) = reader
        .read_path_with_report(&AssetPath::parse("textures/first.texture"))
        .unwrap();
    assert_eq!(first_again_report.cache_status, BundleChunkCacheStatus::Hit);
    assert_eq!(reader.chunk_cache_stats().decoded_chunks, 1);

    let bundle_io = BundleAssetIo::from_bytes_with_loading_policy(
        &bundle,
        BundleChunkLoadingPolicy::OnDemandCachedLimited {
            max_decoded_chunks: 2,
        },
    )
    .unwrap();
    let prefetch = bundle_io
        .prefetch_paths(&["textures/first.texture", "textures/second.texture"])
        .unwrap();
    assert_eq!(prefetch.cache_misses, 2);
    assert_eq!(prefetch.decoded_chunks, vec![0, 1]);
    assert!(prefetch.evicted_chunks.is_empty());
    let third_prefetch = bundle_io.prefetch_path("textures/third.texture").unwrap();
    assert_eq!(third_prefetch.decoded_chunks, vec![2]);
    assert_eq!(third_prefetch.evicted_chunks, vec![0]);
    let stats = bundle_io.chunk_cache_stats();
    assert_eq!(stats.decoded_chunks, 2);
    assert_eq!(stats.cache_evictions, 1);
    assert_eq!(stats.prefetched_chunks, 3);
}

#[test]
fn bundle_reader_reports_corrupted_rle_chunks() {
    let bundle =
        b"NGA_BUNDLE_V2\nname=bad_rle\ncompression=rle\nchunks=1\nchunk|0|0|1|4|rle|1\nentries=0\nDATA\nx";

    assert!(matches!(
        BundleReader::from_bytes(bundle),
        Err(AssetError::Bundle { message })
            if message.contains("rle bundle chunk 0")
                && message.contains("truncated run")
    ));
}

#[test]
fn bundle_writer_writes_file_and_returns_manifest() {
    let path = temp_file("bundle_write_file", "bundle");
    let _ = std::fs::remove_file(&path);
    let shader_id = AssetId::new();
    let texture_id = AssetId::new();
    let bytes = texture_bytes(2, 2, 13);

    let manifest = BundleWriter::write_file(
        &path,
        "persisted_textures",
        CompressionKind::None,
        vec![BundleAsset {
            id: texture_id,
            asset_type: AssetTypeId::of::<Texture>(),
            path: AssetPath::parse("textures/persisted.texture"),
            bytes: bytes.clone(),
            dependencies: vec![shader_id],
        }],
    )
    .unwrap();

    assert_eq!(manifest.name, "persisted_textures");
    assert_eq!(
        manifest.dependencies(texture_id),
        Some([shader_id].as_slice())
    );
    let file_bytes = std::fs::read(&path).unwrap();
    let reader = BundleReader::from_bytes(&file_bytes).unwrap();
    assert_eq!(
        reader
            .read_path(&AssetPath::parse("textures/persisted.texture"))
            .unwrap(),
        bytes
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
#[cfg(feature = "zstd")]
fn bundle_writer_writes_zstd_file_and_returns_manifest() {
    let path = temp_file("bundle_write_zstd_file", "bundle");
    let _ = std::fs::remove_file(&path);
    let texture_id = AssetId::new();
    let first = texture_bytes(2, 2, 31);
    let second = texture_bytes(1, 1, 32);

    let manifest = BundleWriter::write_file(
        &path,
        "persisted_zstd_textures",
        CompressionKind::Zstd,
        vec![
            BundleAsset {
                id: texture_id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/persisted_zstd.texture"),
                bytes: first.clone(),
                dependencies: vec![],
            },
            BundleAsset {
                id: AssetId::new(),
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/persisted_zstd_extra.texture"),
                bytes: second.clone(),
                dependencies: vec![texture_id],
            },
        ],
    )
    .unwrap();

    assert_eq!(manifest.name, "persisted_zstd_textures");
    assert_eq!(manifest.compression, CompressionKind::Zstd);
    assert_eq!(manifest.entries.len(), 2);
    assert_eq!(manifest.dependencies(texture_id), Some([].as_slice()));
    let second_id = manifest
        .entries
        .iter()
        .find(|entry| entry.id != texture_id)
        .map(|entry| entry.id)
        .unwrap();
    assert_eq!(
        manifest.dependencies(second_id),
        Some([texture_id].as_slice())
    );
    assert_eq!(
        manifest.chunk(0).unwrap().compression,
        CompressionKind::Zstd
    );

    let file_bytes = std::fs::read(&path).unwrap();
    let reader = BundleReader::from_bytes_with_loading_policy(
        &file_bytes,
        BundleChunkLoadingPolicy::OnDemandCachedLimited {
            max_decoded_chunks: 1,
        },
    )
    .unwrap();
    assert_eq!(reader.manifest().name, "persisted_zstd_textures");
    assert_eq!(reader.manifest().compression, CompressionKind::Zstd);
    assert_eq!(reader.manifest().entries.len(), 2);
    assert_eq!(reader.chunk_cache_stats().max_decoded_chunks, Some(1));

    let prefetch = reader
        .prefetch_paths(&[
            AssetPath::parse("textures/persisted_zstd.texture"),
            AssetPath::parse("textures/persisted_zstd_extra.texture"),
        ])
        .unwrap();
    assert_eq!(prefetch.decoded_chunks.len(), 1);
    assert_eq!(prefetch.evicted_chunks.len(), 0);

    let (range, report) = reader
        .read_path_range_with_report(&AssetPath::parse("textures/persisted_zstd.texture"), 0, 8)
        .unwrap();
    assert_eq!(range, first[0..8]);
    assert_eq!(report.chunk_compression, CompressionKind::Zstd);
    assert!(matches!(
        BundleAssetIo::from_bytes_with_loading_policy(
            &file_bytes,
            BundleChunkLoadingPolicy::OnDemandCachedLimited {
                max_decoded_chunks: 1,
            },
        ),
        Ok(_)
    ));

    let bundle_io = BundleAssetIo::from_bytes_with_loading_policy(
        &file_bytes,
        BundleChunkLoadingPolicy::OnDemandCachedLimited {
            max_decoded_chunks: 1,
        },
    )
    .unwrap();
    let (read_bytes, read_report) = bundle_io
        .read_with_report("textures/persisted_zstd.texture")
        .unwrap();
    assert_eq!(read_bytes, first);
    assert_eq!(read_report.chunk_compression, CompressionKind::Zstd);
    assert_eq!(
        read_report.path,
        Some(AssetPath::parse("textures/persisted_zstd.texture"))
    );
    let (_, range_report) = bundle_io
        .read_range_with_report(
            "textures/persisted_zstd_extra.texture",
            0,
            second.len() as u64,
        )
        .unwrap();
    assert_eq!(range_report.chunk_compression, CompressionKind::Zstd);
    assert_eq!(
        range_report.path,
        Some(AssetPath::parse("textures/persisted_zstd_extra.texture"))
    );
    assert_eq!(
        bundle_io
            .read_range("textures/persisted_zstd.texture", 0, first.len() as u64)
            .unwrap(),
        first
    );
    assert_eq!(
        bundle_io
            .metadata("textures/persisted_zstd_extra.texture")
            .unwrap()
            .hash,
        Some(content_hash(&second))
    );
    assert_eq!(
        bundle_io
            .read("textures/persisted_zstd_extra.texture")
            .unwrap(),
        second
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn bundle_asset_io_supports_read_range_metadata_list_and_missing_entry_errors() {
    let bytes = texture_bytes(1, 1, 12);
    let (_id, bundle) = texture_bundle("textures/albedo.texture", bytes.clone());
    let io = BundleAssetIo::from_bytes(&bundle).unwrap();

    assert!(io.exists("textures/albedo.texture"));
    assert_eq!(io.read("textures/albedo.texture").unwrap(), bytes);
    assert_eq!(
        io.read_range("textures/albedo.texture", 0, 4).unwrap(),
        bytes[0..4]
    );
    let metadata = io.metadata("textures/albedo.texture").unwrap();
    assert_eq!(metadata.size, bytes.len() as u64);
    assert!(metadata.hash.is_some());
    assert_eq!(
        io.list("textures").unwrap(),
        vec!["textures/albedo.texture"]
    );
    let missing_error = io.read("textures/missing.texture").unwrap_err();
    assert!(matches!(missing_error, AssetIoError::NotFound { .. }));
    assert_eq!(missing_error.path(), "textures/missing.texture");
    assert_eq!(missing_error.action(), AssetIoAction::Read);
}

#[test]
fn composite_asset_io_uses_first_matching_layer_as_override() {
    let base_bytes = texture_bytes(1, 1, 1);
    let override_bytes = texture_bytes(1, 1, 9);
    let (_id, bundle) = texture_bundle("textures/albedo.texture", base_bytes);
    let bundle_io = BundleAssetIo::from_bytes(&bundle).unwrap();
    let source_io = MemoryAssetIo::new()
        .with_file("textures/albedo.texture", override_bytes.clone())
        .with_file("textures/source_only.texture", texture_bytes(1, 1, 2));
    let composite = CompositeAssetIo::new()
        .with_layer(source_io)
        .with_layer(bundle_io);

    assert_eq!(
        composite.read("textures/albedo.texture").unwrap(),
        override_bytes
    );
    assert!(composite.exists("textures/source_only.texture"));
    assert_eq!(
        composite.layers(),
        vec![
            AssetIoLayerInfo::new("layer_0", AssetIoLayerKind::Custom, 0),
            AssetIoLayerInfo::new("layer_1", AssetIoLayerKind::Custom, 1),
        ]
    );
    let list = composite.list("textures").unwrap();
    assert_eq!(
        list,
        vec!["textures/albedo.texture", "textures/source_only.texture"]
    );
}

#[test]
fn composite_asset_io_reports_named_source_mod_patch_bundle_precedence() {
    let source_bytes = texture_bytes(1, 1, 10);
    let mod_bytes = texture_bytes(1, 1, 20);
    let mod_only_bytes = texture_bytes(1, 1, 21);
    let patch_bytes = texture_bytes(1, 1, 30);
    let base_only_bytes = texture_bytes(1, 1, 40);
    let base_bundle = texture_bundle_io(
        "base_textures",
        vec![
            ("textures/albedo.texture", texture_bytes(1, 1, 1)),
            ("textures/base_only.texture", base_only_bytes.clone()),
        ],
    );
    let patch_io = MemoryAssetIo::new()
        .with_file("textures/albedo.texture", patch_bytes.clone())
        .with_file("textures/patch_only.texture", texture_bytes(1, 1, 31));
    let mod_io = MemoryAssetIo::new()
        .with_file("textures/albedo.texture", mod_bytes.clone())
        .with_file("textures/mod_only.texture", mod_only_bytes.clone());
    let source_io = MemoryAssetIo::new().with_file("textures/albedo.texture", source_bytes.clone());
    let composite = CompositeAssetIo::new()
        .with_named_layer("source", AssetIoLayerKind::Source, source_io)
        .with_named_layer("mod:hires", AssetIoLayerKind::Mod, mod_io)
        .with_named_layer("patch:day0", AssetIoLayerKind::Patch, patch_io)
        .with_named_layer("base_bundle", AssetIoLayerKind::BaseBundle, base_bundle);

    let (bytes, resolution) = composite
        .read_with_diagnostics("textures/albedo.texture")
        .unwrap();
    assert_eq!(bytes, source_bytes);
    assert_eq!(
        resolution.layer,
        AssetIoLayerInfo::new("source", AssetIoLayerKind::Source, 0)
    );
    let (metadata, resolution) = composite
        .metadata_with_diagnostics("textures/albedo.texture")
        .unwrap();
    assert_eq!(metadata.size, source_bytes.len() as u64);
    assert_eq!(
        resolution.layer,
        AssetIoLayerInfo::new("source", AssetIoLayerKind::Source, 0)
    );

    let (bytes, resolution) = composite
        .read_with_diagnostics("textures/mod_only.texture")
        .unwrap();
    assert_eq!(bytes, mod_only_bytes);
    assert_eq!(resolution.layer.name, "mod:hires");
    assert_eq!(resolution.layer.kind, AssetIoLayerKind::Mod);

    let (metadata, resolution) = composite
        .metadata_with_diagnostics("textures/base_only.texture")
        .unwrap();
    assert_eq!(metadata.size, base_only_bytes.len() as u64);
    assert_eq!(resolution.layer.name, "base_bundle");
    assert_eq!(resolution.layer.kind, AssetIoLayerKind::BaseBundle);

    let resolution = composite.resolve("textures/patch_only.texture").unwrap();
    assert_eq!(resolution.layer.name, "patch:day0");
    assert_eq!(resolution.layer.priority, 2);
    assert!(composite.resolve("textures/missing.texture").is_none());
}

#[test]
fn composite_asset_io_list_diagnostics_deduplicate_shadowed_paths_across_layers() {
    let base_bundle = texture_bundle_io(
        "base_textures",
        vec![
            ("textures/albedo.texture", texture_bytes(1, 1, 1)),
            ("textures/base_only.texture", texture_bytes(1, 1, 2)),
            ("textures/patch_only.texture", texture_bytes(1, 1, 3)),
        ],
    );
    let patch_io = MemoryAssetIo::new()
        .with_file("textures/albedo.texture", texture_bytes(1, 1, 30))
        .with_file("textures/patch_only.texture", texture_bytes(1, 1, 31));
    let mod_io = MemoryAssetIo::new()
        .with_file("textures/patch_only.texture", texture_bytes(1, 1, 20))
        .with_file("textures/mod_only.texture", texture_bytes(1, 1, 21));
    let source_io =
        MemoryAssetIo::new().with_file("textures/source_only.texture", texture_bytes(1, 1, 10));
    let composite = CompositeAssetIo::new()
        .with_named_layer("source", AssetIoLayerKind::Source, source_io)
        .with_named_layer("mod", AssetIoLayerKind::Mod, mod_io)
        .with_named_layer("patch", AssetIoLayerKind::Patch, patch_io)
        .with_named_layer("base_bundle", AssetIoLayerKind::BaseBundle, base_bundle);

    let entries = composite.list_with_diagnostics("textures").unwrap();
    let served_by = entries
        .iter()
        .map(|entry| {
            (
                entry.path.as_str(),
                entry.layer.name.as_str(),
                entry.layer.kind,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        served_by,
        vec![
            ("textures/albedo.texture", "patch", AssetIoLayerKind::Patch,),
            (
                "textures/base_only.texture",
                "base_bundle",
                AssetIoLayerKind::BaseBundle,
            ),
            ("textures/mod_only.texture", "mod", AssetIoLayerKind::Mod),
            ("textures/patch_only.texture", "mod", AssetIoLayerKind::Mod),
            (
                "textures/source_only.texture",
                "source",
                AssetIoLayerKind::Source,
            ),
        ]
    );
    assert_eq!(
        composite.list("textures").unwrap(),
        served_by
            .iter()
            .map(|(path, _, _)| (*path).to_owned())
            .collect::<Vec<_>>()
    );
}

#[test]
fn asset_package_registry_persists_order_enabled_state_and_reports_conflicts() {
    let base_albedo = texture_bytes(1, 1, 1);
    let patch_albedo = texture_bytes(1, 1, 2);
    let mod_albedo = texture_bytes(1, 1, 3);
    let mod_only = texture_bytes(1, 1, 4);
    let disabled_albedo = texture_bytes(1, 1, 99);

    let (base, base_bundle, _) = texture_package(
        "base",
        AssetIoLayerKind::BaseBundle,
        30,
        BundleId(30),
        "packages/base.nga_bundle",
        vec![
            ("textures/albedo.texture", base_albedo),
            ("textures/base_only.texture", texture_bytes(1, 1, 10)),
        ],
    );
    let (patch, patch_bundle, _) = texture_package(
        "patch_day0",
        AssetIoLayerKind::Patch,
        20,
        BundleId(20),
        "packages/patch_day0.nga_bundle",
        vec![
            ("textures/albedo.texture", patch_albedo),
            ("textures/patch_only.texture", texture_bytes(1, 1, 20)),
        ],
    );
    let (package_mod, mod_bundle, _) = texture_package(
        "mod_hires",
        AssetIoLayerKind::Mod,
        10,
        BundleId(10),
        "packages/mod_hires.nga_bundle",
        vec![
            ("textures/albedo.texture", mod_albedo.clone()),
            ("textures/mod_only.texture", mod_only.clone()),
        ],
    );
    let (mut disabled, _disabled_bundle, _) = texture_package(
        "disabled_mod",
        AssetIoLayerKind::Mod,
        0,
        BundleId(40),
        "packages/disabled.nga_bundle",
        vec![("textures/albedo.texture", disabled_albedo)],
    );
    disabled.enabled = false;

    let registry = AssetPackageRegistry::new(vec![
        base.clone(),
        disabled.clone(),
        patch.clone(),
        package_mod,
    ])
    .unwrap();
    assert_eq!(
        registry
            .packages()
            .iter()
            .map(|package| package.name.as_str())
            .collect::<Vec<_>>(),
        vec!["disabled_mod", "mod_hires", "patch_day0", "base"]
    );
    assert_eq!(registry.enabled_packages().count(), 3);

    let text = registry.to_text();
    let restored = AssetPackageRegistry::from_text(&text).unwrap();
    assert_eq!(restored, registry);

    let report = restored.conflict_report();
    assert!(report.has_conflicts());
    assert_eq!(report.conflicts.len(), 1);
    let conflict = &report.conflicts[0];
    assert_eq!(conflict.path, AssetPath::parse("textures/albedo.texture"));
    assert_eq!(conflict.winner.name, "mod_hires");
    assert_eq!(conflict.winner.kind, AssetIoLayerKind::Mod);
    assert_eq!(
        conflict
            .shadowed
            .iter()
            .map(|layer| (layer.name.as_str(), layer.kind))
            .collect::<Vec<_>>(),
        vec![
            ("patch_day0", AssetIoLayerKind::Patch),
            ("base", AssetIoLayerKind::BaseBundle),
        ]
    );

    let bundles = std::collections::HashMap::from([
        ("packages/base.nga_bundle".to_owned(), base_bundle),
        ("packages/patch_day0.nga_bundle".to_owned(), patch_bundle),
        ("packages/mod_hires.nga_bundle".to_owned(), mod_bundle),
    ]);
    let composite = restored
        .build_composite_io(|package| {
            bundles
                .get(&package.bundle_path)
                .cloned()
                .ok_or_else(|| AssetError::Bundle {
                    message: format!("missing package payload `{}`", package.bundle_path),
                })
        })
        .unwrap();

    let (bytes, resolution) = composite
        .read_with_diagnostics("textures/albedo.texture")
        .unwrap();
    assert_eq!(bytes, mod_albedo);
    assert_eq!(resolution.layer.name, "mod_hires");
    assert_eq!(resolution.layer.kind, AssetIoLayerKind::Mod);
    let (metadata, resolution) = composite
        .metadata_with_diagnostics("textures/albedo.texture")
        .unwrap();
    assert_eq!(metadata.size, mod_albedo.len() as u64);
    assert_eq!(resolution.layer.name, "mod_hires");
    assert_eq!(resolution.layer.kind, AssetIoLayerKind::Mod);
    assert_eq!(
        composite
            .read_with_diagnostics("textures/mod_only.texture")
            .unwrap()
            .0,
        mod_only
    );
    assert!(composite
        .resolve("textures/disabled_only.texture")
        .is_none());
}

#[test]
fn asset_package_registry_reports_invalid_metadata_and_payload_mismatch() {
    assert!(matches!(
        AssetPackageRegistry::from_text("not a package registry"),
        Err(AssetError::Bundle { message }) if message.contains("invalid asset package registry header")
    ));
    assert!(matches!(
        AssetPackageRegistry::from_text("NGA_ASSET_PACKAGE_REGISTRY_V1\npackages=abc"),
        Err(AssetError::Bundle { message }) if message.contains("invalid asset package count")
    ));
    assert!(matches!(
        AssetPackageRegistry::from_text("NGA_ASSET_PACKAGE_REGISTRY_V1"),
        Err(AssetError::Bundle { message }) if message.contains("missing `packages=` line")
    ));
    assert!(matches!(
        AssetPackageRegistry::from_text("NGA_ASSET_PACKAGE_REGISTRY_V1\npackage=1"),
        Err(AssetError::Bundle { message }) if message.contains("expected `packages=` line")
    ));
    assert!(matches!(
        AssetPackageRegistry::from_text(
            "NGA_ASSET_PACKAGE_REGISTRY_V1\npackages=1\npackage|1|0|true|patch|patch|packages/patch.nga_bundle|2\nNGA_BUNDLE_V2"
        ),
        Err(AssetError::Bundle { message }) if message.contains("manifest is truncated")
    ));
    assert!(matches!(
        AssetPackageRegistry::from_text(&format!(
            "NGA_ASSET_PACKAGE_REGISTRY_V3\npackages=1\npackage|1|0|true|patch|patch|packages/patch.nga_bundle|1|1||{}",
            usize::MAX
        )),
        Err(AssetError::Bundle { message }) if message.contains("asset package manifest line count overflow")
    ));
    assert!(matches!(
        AssetPackageRegistry::from_text(
            "NGA_ASSET_PACKAGE_REGISTRY_V3\npackages=1\npackage|1|0|true|patch|patch|packages/patch.nga_bundle|1|1||5\nNGA_BUNDLE_V2\nname=patch\ncompression=none\nchunks=0\nentries=0\nextra"
        ),
        Err(AssetError::Bundle { message }) if message.contains("unexpected trailing asset package registry data")
    ));
    assert!(matches!(
        AssetPackageRegistry::from_text(
            "NGA_ASSET_PACKAGE_REGISTRY_V3\npackages=1\npackage|1|0|true|patch|patch|packages/patch.nga_bundle|1|1||6\nNGA_BUNDLE_V2\nname=patch\ncompression=none\nchunks=0\nentries=0\nextra"
        ),
        Err(AssetError::Bundle { message }) if message.contains("unexpected trailing bundle manifest data")
    ));

    let (valid, _valid_bundle, _) = texture_package(
        "valid",
        AssetIoLayerKind::Patch,
        0,
        BundleId(1),
        "packages/valid.nga_bundle",
        vec![("textures/valid.texture", texture_bytes(1, 1, 1))],
    );
    let dependency_registry = AssetPackageRegistry::new(vec![valid
        .clone()
        .with_package_dependency(AssetPackageDependency::new("base_dependency", 2))])
    .unwrap();
    let malformed_dependency_text = dependency_registry
        .to_text()
        .replace("base_dependency:2:", "base_dependency:2");
    assert!(matches!(
        AssetPackageRegistry::from_text(&malformed_dependency_text),
        Err(AssetError::Bundle { message }) if message.contains("invalid asset package dependency field")
    ));
    let invalid_bundle_id_text =
        dependency_registry
            .to_text()
            .replacen("package|1|", "package|abc|", 1);
    assert!(matches!(
        AssetPackageRegistry::from_text(&invalid_bundle_id_text),
        Err(AssetError::Bundle { message }) if message.contains("invalid asset package bundle id")
    ));
    let invalid_priority_text =
        dependency_registry
            .to_text()
            .replacen("package|1|0|", "package|1|abc|", 1);
    assert!(matches!(
        AssetPackageRegistry::from_text(&invalid_priority_text),
        Err(AssetError::Bundle { message }) if message.contains("invalid asset package priority")
    ));
    let invalid_version_text =
        dependency_registry
            .to_text()
            .replacen("|1|1|base_dependency", "|abc|1|base_dependency", 1);
    assert!(matches!(
        AssetPackageRegistry::from_text(&invalid_version_text),
        Err(AssetError::Bundle { message }) if message.contains("invalid asset package version")
    ));
    let invalid_minimum_runtime_text =
        dependency_registry
            .to_text()
            .replacen("|1|1|base_dependency", "|1|abc|base_dependency", 1);
    assert!(matches!(
        AssetPackageRegistry::from_text(&invalid_minimum_runtime_text),
        Err(AssetError::Bundle { message }) if message.contains("invalid asset package minimum runtime version")
    ));
    let invalid_enabled_text =
        dependency_registry
            .to_text()
            .replacen("package|1|0|true|", "package|1|0|maybe|", 1);
    assert!(matches!(
        AssetPackageRegistry::from_text(&invalid_enabled_text),
        Err(AssetError::Bundle { message }) if message.contains("invalid asset package enabled")
    ));
    let missing_package_line_text = dependency_registry
        .to_text()
        .replace("packages=1", "packages=2");
    assert!(matches!(
        AssetPackageRegistry::from_text(&missing_package_line_text),
        Err(AssetError::Bundle { message }) if message.contains("missing asset package line 1")
    ));
    let invalid_package_line_text = dependency_registry
        .to_text()
        .replacen("package|", "packagx|", 1);
    assert!(matches!(
        AssetPackageRegistry::from_text(&invalid_package_line_text),
        Err(AssetError::Bundle { message }) if message.contains("invalid asset package line 0")
    ));
    let (mut duplicate_name, _duplicate_bundle, _) = texture_package(
        "valid",
        AssetIoLayerKind::Mod,
        1,
        BundleId(2),
        "packages/duplicate.nga_bundle",
        vec![("textures/other.texture", texture_bytes(1, 1, 2))],
    );
    duplicate_name.name = valid.name.clone();
    assert!(matches!(
        AssetPackageRegistry::new(vec![valid.clone(), duplicate_name]),
        Err(AssetError::Bundle { message }) if message.contains("duplicate asset package name")
    ));
    let mut duplicate_id = valid.clone();
    duplicate_id.name = "duplicate_id".to_owned();
    duplicate_id.priority = 1;
    assert!(matches!(
        AssetPackageRegistry::new(vec![valid.clone(), duplicate_id]),
        Err(AssetError::Bundle { message }) if message.contains("duplicate asset package bundle id")
    ));
    let mut duplicate_priority = valid.clone();
    duplicate_priority.name = "duplicate_priority".to_owned();
    duplicate_priority.bundle_id = BundleId(22);
    assert!(matches!(
        AssetPackageRegistry::new(vec![valid.clone(), duplicate_priority]),
        Err(AssetError::Bundle { message }) if message.contains("duplicate asset package priority")
    ));
    let mut empty_path = valid.clone();
    empty_path.name = "empty_path".to_owned();
    empty_path.bundle_id = BundleId(23);
    empty_path.priority = 23;
    empty_path.bundle_path.clear();
    assert!(matches!(
        AssetPackageRegistry::new(vec![empty_path]),
        Err(AssetError::Bundle { message }) if message.contains("bundle path cannot be empty")
    ));
    let mut zero_version = valid.clone();
    zero_version.name = "zero_version".to_owned();
    zero_version.bundle_id = BundleId(24);
    zero_version.priority = 24;
    zero_version.package_version = 0;
    assert!(matches!(
        AssetPackageRegistry::new(vec![zero_version]),
        Err(AssetError::Bundle { message }) if message.contains("version must be greater than zero")
    ));
    let mut zero_runtime = valid.clone();
    zero_runtime.name = "zero_runtime".to_owned();
    zero_runtime.bundle_id = BundleId(25);
    zero_runtime.priority = 25;
    zero_runtime.minimum_runtime_version = 0;
    assert!(matches!(
        AssetPackageRegistry::new(vec![zero_runtime]),
        Err(AssetError::Bundle { message }) if message.contains("minimum runtime version must be greater than zero")
    ));
    let mut duplicate_manifest = valid.manifest.clone();
    duplicate_manifest
        .entries
        .push(duplicate_manifest.entries[0].clone());
    assert!(matches!(
        AssetPackageRegistry::new(vec![AssetPackageRecord::new(
            BundleId(26),
            "duplicate_manifest",
            AssetIoLayerKind::Patch,
            26,
            true,
            "packages/duplicate_manifest.nga_bundle",
            duplicate_manifest,
        )]),
        Err(AssetError::Bundle { message }) if message.contains("manifest has duplicate path")
    ));
    let invalid_kind_text = AssetPackageRegistry::new(vec![valid.clone()])
        .unwrap()
        .to_text()
        .replace("|patch|", "|invalid_kind|");
    assert!(matches!(
        AssetPackageRegistry::from_text(&invalid_kind_text),
        Err(AssetError::Bundle { message }) if message.contains("unknown asset package layer kind")
    ));

    let (other, other_bundle, _) = texture_package(
        "other",
        AssetIoLayerKind::Patch,
        0,
        BundleId(3),
        "packages/other.nga_bundle",
        vec![("textures/other.texture", texture_bytes(1, 1, 3))],
    );
    let registry = AssetPackageRegistry::new(vec![valid]).unwrap();
    assert!(matches!(
        registry.build_composite_io(|_| Ok(other_bundle.clone())),
        Err(AssetError::Bundle { message }) if message.contains("payload manifest does not match")
    ));

    let missing_registry = AssetPackageRegistry::new(vec![other]).unwrap();
    assert!(matches!(
        missing_registry.build_composite_io(|package| Err(AssetError::Bundle {
            message: format!("missing package payload `{}`", package.bundle_path),
        })),
        Err(AssetError::Bundle { message }) if message.contains("missing package payload")
    ));
}

#[test]
fn asset_package_registry_rejects_registry_separator_tokens() {
    let (_valid, _valid_bundle, _) = texture_package(
        "valid",
        AssetIoLayerKind::Patch,
        0,
        BundleId(1),
        "packages/valid.nga_bundle",
        vec![("textures/valid.texture", texture_bytes(1, 1, 1))],
    );

    let (mut separator_name, separator_bundle, _) = texture_package(
        "separator_name",
        AssetIoLayerKind::Patch,
        1,
        BundleId(2),
        "packages/separator_name.nga_bundle",
        vec![("textures/other.texture", texture_bytes(1, 1, 2))],
    );
    separator_name.name = "separator|name".to_owned();
    assert!(matches!(
        AssetPackageRegistry::new(vec![separator_name]),
        Err(AssetError::Bundle { message }) if message.contains("package name") && message.contains("registry separators")
    ));

    let (mut separator_path, _separator_bundle, _) = texture_package(
        "separator_path",
        AssetIoLayerKind::Patch,
        2,
        BundleId(3),
        "packages/separator_path.nga_bundle",
        vec![("textures/other.texture", texture_bytes(1, 1, 3))],
    );
    separator_path.bundle_path = "packages/sep|path.nga_bundle".to_owned();
    assert!(matches!(
        AssetPackageRegistry::new(vec![separator_path]),
        Err(AssetError::Bundle { message }) if message.contains("package bundle path") && message.contains("registry separators")
    ));

    let separator_registry = AssetPackageRegistry::new(vec![{
        let mut record = texture_package(
            "separator_text",
            AssetIoLayerKind::Patch,
            3,
            BundleId(4),
            "packages/separator_text.nga_bundle",
            vec![("textures/other.texture", texture_bytes(1, 1, 4))],
        )
        .0;
        record.name = "separator\ntext".to_owned();
        record
    }]);
    assert!(matches!(
        separator_registry,
        Err(AssetError::Bundle { message }) if message.contains("package name") && message.contains("registry separators")
    ));

    let _ = separator_bundle;
}

#[test]
fn asset_package_artifact_store_installs_builds_and_removes_package_files() {
    let root = temp_dir("package_artifacts");
    let _ = std::fs::remove_dir_all(&root);
    let store = AssetPackageArtifactStore::new(&root);
    let mut registry = AssetPackageRegistry::default();
    let base_albedo = texture_bytes(1, 1, 3);
    let patch_albedo = texture_bytes(1, 1, 9);
    let (_base_record, base_bundle, _) = texture_package(
        "artifact_base",
        AssetIoLayerKind::BaseBundle,
        10,
        BundleId(10),
        "unused/base.bundle",
        vec![
            ("textures/albedo.texture", base_albedo),
            ("textures/base_only.texture", texture_bytes(1, 1, 4)),
        ],
    );
    let (_patch_record, patch_bundle, _) = texture_package(
        "artifact_patch",
        AssetIoLayerKind::Patch,
        0,
        BundleId(11),
        "unused/patch.bundle",
        vec![
            ("textures/albedo.texture", patch_albedo.clone()),
            ("textures/patch_only.texture", texture_bytes(1, 1, 10)),
        ],
    );

    let base_install = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                BundleId(10),
                "artifact_base",
                AssetIoLayerKind::BaseBundle,
                10,
                "base/artifact_base.bundle",
            )
            .with_package_version(2),
            &base_bundle,
        )
        .unwrap();
    assert!(base_install.artifact_path.exists());
    assert_eq!(base_install.payload_size, base_bundle.len() as u64);
    assert_eq!(base_install.payload_hash, content_hash(&base_bundle));
    assert!(base_install.replaced.is_none());

    let patch_install = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                BundleId(11),
                "artifact_patch",
                AssetIoLayerKind::Patch,
                0,
                "patches/artifact_patch.bundle",
            ),
            &patch_bundle,
        )
        .unwrap();
    assert!(patch_install.conflicts.has_conflicts());
    assert_eq!(
        registry
            .packages()
            .iter()
            .map(|package| package.name.as_str())
            .collect::<Vec<_>>(),
        vec!["artifact_patch", "artifact_base"]
    );

    let report = store.verify_registry(&registry).unwrap();
    assert!(report.all_available());
    assert_eq!(report.packages.len(), 2);
    assert!(report
        .packages
        .iter()
        .all(|status| status.manifest_matches == Some(true)));

    let composite = store.build_composite_io(&registry).unwrap();
    assert_eq!(
        composite.read("textures/albedo.texture").unwrap(),
        patch_albedo
    );

    #[cfg(feature = "zstd")]
    {
        let zstd_preview = BundleWriter::build_bytes_with_options(
            "artifact_zstd_preview",
            BundleBuildOptions::new(CompressionKind::Zstd)
                .with_chunk_policy(BundleChunkPartitionPolicy::MaxUncompressedBytes(1)),
            vec![
                BundleAsset {
                    id: AssetId::new(),
                    asset_type: AssetTypeId::of::<Texture>(),
                    path: AssetPath::parse("textures/zstd_base.texture"),
                    bytes: texture_bytes(1, 1, 8),
                    dependencies: Vec::new(),
                },
                BundleAsset {
                    id: AssetId::new(),
                    asset_type: AssetTypeId::of::<Texture>(),
                    path: AssetPath::parse("textures/zstd_patch.texture"),
                    bytes: texture_bytes(1, 1, 9),
                    dependencies: Vec::new(),
                },
            ],
        )
        .unwrap();
        let zstd_install = store
            .install_package_bytes(
                &mut registry,
                AssetPackageInstallRequest::new(
                    BundleId(13),
                    "artifact_zstd",
                    AssetIoLayerKind::Patch,
                    1,
                    "patches/artifact_zstd.bundle",
                ),
                &zstd_preview,
            )
            .unwrap();
        assert!(zstd_install.artifact_path.exists());
        assert_eq!(zstd_install.payload_size, zstd_preview.len() as u64);
        assert_eq!(zstd_install.payload_hash, content_hash(&zstd_preview));
        let zstd_composite = store.build_composite_io(&registry).unwrap();
        assert_eq!(
            zstd_composite.read("textures/zstd_patch.texture").unwrap(),
            texture_bytes(1, 1, 9)
        );
        assert_eq!(
            zstd_composite
                .metadata("textures/zstd_patch.texture")
                .unwrap()
                .hash,
            Some(content_hash(&texture_bytes(1, 1, 9)))
        );
    }

    let registry_path = root.join("packages.txt");
    registry.save_to_file(&registry_path).unwrap();
    assert_eq!(
        AssetPackageRegistry::load_from_file(&registry_path).unwrap(),
        registry
    );

    let removed = store
        .remove_package(&mut registry, "artifact_patch", true)
        .unwrap();
    assert_eq!(removed.removed.name, "artifact_patch");
    assert!(removed.artifact_removed);
    assert!(!removed.artifact_path.exists());
    assert!(matches!(
        store.load_package_bytes(&removed.removed),
        Err(AssetError::Io { message }) if message.contains("failed to read") && message.contains("artifact_patch.bundle")
    ));
    assert!(!removed.conflicts.has_conflicts());
    assert_eq!(registry.packages().len(), 2);
    assert!(store.verify_registry(&registry).unwrap().all_available());

    let keep_install = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                BundleId(12),
                "artifact_keep",
                AssetIoLayerKind::Patch,
                0,
                "patches/artifact_keep.bundle",
            ),
            &patch_bundle,
        )
        .unwrap();
    assert!(keep_install.artifact_path.exists());

    let kept = store
        .remove_package(&mut registry, "artifact_keep", false)
        .unwrap();
    assert_eq!(kept.removed.name, "artifact_keep");
    assert!(!kept.artifact_removed);
    assert!(kept.artifact_path.exists());
    assert_eq!(
        store.load_package_bytes(&kept.removed).unwrap(),
        patch_bundle
    );
    assert_eq!(registry.packages().len(), 2);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[cfg(feature = "zstd")]
fn asset_package_artifact_store_installs_and_builds_zstd_audio_package_files() {
    let root = temp_dir("package_artifacts_audio_zstd");
    let _ = std::fs::remove_dir_all(&root);
    let store = AssetPackageArtifactStore::new(&root);
    let mut registry = AssetPackageRegistry::default();

    let audio_bytes = ogg_vorbis_audio_bytes(44_100, 2);
    let audio_bundle = BundleWriter::build_bytes_with_options(
        "artifact_audio_zstd",
        BundleBuildOptions::new(CompressionKind::Zstd)
            .with_chunk_policy(BundleChunkPartitionPolicy::MaxUncompressedBytes(1)),
        vec![BundleAsset {
            id: AssetId::new(),
            asset_type: AssetTypeId::of::<AudioClip>(),
            path: AssetPath::parse("audio/voice.ogg"),
            bytes: audio_bytes.clone(),
            dependencies: Vec::new(),
        }],
    )
    .unwrap();

    let install = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                BundleId(14),
                "artifact_audio_zstd",
                AssetIoLayerKind::Patch,
                0,
                "patches/artifact_audio_zstd.bundle",
            ),
            &audio_bundle,
        )
        .unwrap();
    assert!(install.artifact_path.exists());
    assert_eq!(install.payload_size, audio_bundle.len() as u64);
    assert_eq!(install.payload_hash, content_hash(&audio_bundle));

    let composite = store.build_composite_io(&registry).unwrap();
    let (read_bytes, read_report) = composite.read_with_diagnostics("audio/voice.ogg").unwrap();
    assert_eq!(read_bytes, audio_bytes);
    assert_eq!(read_report.layer.name, "artifact_audio_zstd");
    assert_eq!(read_report.layer.kind, AssetIoLayerKind::Patch);
    assert_eq!(
        composite.read_range("audio/voice.ogg", 0, 4).unwrap(),
        audio_bytes[..4].to_vec()
    );
    assert_eq!(
        composite.metadata("audio/voice.ogg").unwrap().hash,
        Some(content_hash(&audio_bytes))
    );

    let registry_path = root.join("audio_packages.txt");
    registry.save_to_file(&registry_path).unwrap();
    let loaded_registry = AssetPackageRegistry::load_from_file(&registry_path).unwrap();
    assert_eq!(loaded_registry, registry);
    assert_eq!(
        store
            .build_composite_io(&loaded_registry)
            .unwrap()
            .read("audio/voice.ogg")
            .unwrap(),
        audio_bytes
    );

    let removed = store
        .remove_package(&mut registry, "artifact_audio_zstd", true)
        .unwrap();
    assert_eq!(removed.removed.name, "artifact_audio_zstd");
    assert!(removed.artifact_removed);
    assert!(!removed.artifact_path.exists());
    assert!(store.verify_registry(&registry).unwrap().all_available());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn asset_package_artifact_store_replaces_packages_with_matching_names() {
    let root = temp_dir("package_artifacts_replace");
    let _ = std::fs::remove_dir_all(&root);
    let store = AssetPackageArtifactStore::new(&root);
    let mut registry = AssetPackageRegistry::default();

    let (_original_record, original_bundle, _) = texture_package(
        "artifact_replace",
        AssetIoLayerKind::Patch,
        4,
        BundleId(40),
        "patches/artifact_replace_v1.bundle",
        vec![("textures/replace.texture", texture_bytes(1, 1, 7))],
    );
    let original_install = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                BundleId(40),
                "artifact_replace",
                AssetIoLayerKind::Patch,
                4,
                "patches/artifact_replace_v1.bundle",
            ),
            &original_bundle,
        )
        .unwrap();
    assert!(original_install.replaced.is_none());

    let (_replacement_record, replacement_bundle, _) = texture_package(
        "artifact_replace",
        AssetIoLayerKind::Patch,
        7,
        BundleId(41),
        "patches/artifact_replace_v2.bundle",
        vec![("textures/replace.texture", texture_bytes(1, 1, 13))],
    );
    let replacement_install = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                BundleId(41),
                "artifact_replace",
                AssetIoLayerKind::Patch,
                7,
                "patches/artifact_replace_v2.bundle",
            ),
            &replacement_bundle,
        )
        .unwrap();
    let replaced = replacement_install.replaced.expect("expected replacement");
    assert_eq!(replaced.bundle_id, BundleId(40));
    assert_eq!(replaced.name, "artifact_replace");
    assert_eq!(replacement_install.record.bundle_id, BundleId(41));
    assert_eq!(replacement_install.record.name, "artifact_replace");
    assert_eq!(
        store
            .load_package_bytes(&replacement_install.record)
            .unwrap(),
        replacement_bundle
    );
    assert_eq!(
        registry
            .packages()
            .iter()
            .map(|package| (package.name.as_str(), package.bundle_id))
            .collect::<Vec<_>>(),
        vec![("artifact_replace", BundleId(41))]
    );
    assert!(store.verify_registry(&registry).unwrap().all_available());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn asset_package_artifact_store_replaces_packages_with_matching_bundle_ids() {
    let root = temp_dir("package_artifacts_replace_bundle_id");
    let _ = std::fs::remove_dir_all(&root);
    let store = AssetPackageArtifactStore::new(&root);
    let mut registry = AssetPackageRegistry::default();

    let (_original_record, original_bundle, _) = texture_package(
        "artifact_bundle_id_base",
        AssetIoLayerKind::Patch,
        4,
        BundleId(50),
        "patches/artifact_bundle_id_v1.bundle",
        vec![("textures/replace.texture", texture_bytes(1, 1, 7))],
    );
    let _ = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                BundleId(50),
                "artifact_bundle_id_base",
                AssetIoLayerKind::Patch,
                4,
                "patches/artifact_bundle_id_v1.bundle",
            ),
            &original_bundle,
        )
        .unwrap();

    let (_replacement_record, replacement_bundle, _) = texture_package(
        "artifact_bundle_id_alias",
        AssetIoLayerKind::Patch,
        7,
        BundleId(50),
        "patches/artifact_bundle_id_v2.bundle",
        vec![("textures/replace.texture", texture_bytes(1, 1, 13))],
    );
    let replacement_install = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                BundleId(50),
                "artifact_bundle_id_alias",
                AssetIoLayerKind::Patch,
                7,
                "patches/artifact_bundle_id_v2.bundle",
            ),
            &replacement_bundle,
        )
        .unwrap();
    let replaced = replacement_install.replaced.expect("expected replacement");
    assert_eq!(replaced.bundle_id, BundleId(50));
    assert_eq!(replaced.name, "artifact_bundle_id_base");
    assert_eq!(replacement_install.record.bundle_id, BundleId(50));
    assert_eq!(replacement_install.record.name, "artifact_bundle_id_alias");
    assert_eq!(
        store
            .load_package_bytes(&replacement_install.record)
            .unwrap(),
        replacement_bundle
    );
    assert_eq!(
        registry
            .packages()
            .iter()
            .map(|package| (package.name.as_str(), package.bundle_id))
            .collect::<Vec<_>>(),
        vec![("artifact_bundle_id_alias", BundleId(50))]
    );
    assert!(store.verify_registry(&registry).unwrap().all_available());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn asset_package_artifact_store_rejects_bundle_path_escape() {
    let root = temp_dir("package_artifacts_escape");
    let _ = std::fs::remove_dir_all(&root);
    let store = AssetPackageArtifactStore::new(&root);
    let mut registry = AssetPackageRegistry::default();

    let (_record, bundle, _) = texture_package(
        "artifact_escape",
        AssetIoLayerKind::Patch,
        4,
        BundleId(60),
        "patches/artifact_escape.bundle",
        vec![("textures/escape.texture", texture_bytes(1, 1, 21))],
    );

    assert!(matches!(
        store.artifact_path("../escape.bundle"),
        Err(AssetError::Bundle { message }) if message.contains("must be relative") || message.contains("cannot escape the artifact root")
    ));
    assert!(matches!(
        store.install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                BundleId(60),
                "artifact_escape",
                AssetIoLayerKind::Patch,
                4,
                "../escape.bundle",
            ),
            &bundle,
        ),
        Err(AssetError::Bundle { message }) if message.contains("cannot escape the artifact root")
    ));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn asset_package_artifact_store_rejects_absolute_bundle_path() {
    let root = temp_dir("package_artifacts_absolute");
    let _ = std::fs::remove_dir_all(&root);
    let store = AssetPackageArtifactStore::new(&root);
    let mut registry = AssetPackageRegistry::default();

    let (_record, bundle, _) = texture_package(
        "artifact_absolute",
        AssetIoLayerKind::Patch,
        4,
        BundleId(61),
        "patches/artifact_absolute.bundle",
        vec![("textures/absolute.texture", texture_bytes(1, 1, 22))],
    );

    assert!(matches!(
        store.artifact_path("C:/escape.bundle"),
        Err(AssetError::Bundle { message }) if message.contains("must be relative")
    ));
    assert!(matches!(
        store.install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                BundleId(61),
                "artifact_absolute",
                AssetIoLayerKind::Patch,
                4,
                "C:/escape.bundle",
            ),
            &bundle,
        ),
        Err(AssetError::Bundle { message }) if message.contains("must be relative")
    ));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn asset_package_artifact_store_reports_missing_package_removal() {
    let root = temp_dir("package_artifacts_missing_remove");
    let _ = std::fs::remove_dir_all(&root);
    let store = AssetPackageArtifactStore::new(&root);
    let mut registry = AssetPackageRegistry::default();

    assert!(matches!(
        store.remove_package(&mut registry, "missing_package", true),
        Err(AssetError::Bundle { message }) if message.contains("asset package `missing_package` is not registered")
    ));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn asset_server_activation_from_artifacts_reports_missing_and_mismatched_payloads() {
    let root = temp_dir("package_artifact_activation");
    let _ = std::fs::remove_dir_all(&root);
    let store = AssetPackageArtifactStore::new(&root);
    let mut registry = AssetPackageRegistry::default();
    let (base_record_seed, base_bundle, base_ids) = texture_package(
        "artifact_runtime_base",
        AssetIoLayerKind::BaseBundle,
        10,
        BundleId(20),
        "unused/runtime_base.bundle",
        vec![("textures/runtime_artifact.texture", texture_bytes(1, 1, 5))],
    );
    let (patch_record, patch_bundle, _) = texture_package(
        "artifact_runtime_patch",
        AssetIoLayerKind::Patch,
        0,
        BundleId(21),
        "patches/runtime_patch.bundle",
        vec![("textures/runtime_artifact.texture", texture_bytes(1, 1, 8))],
    );
    let entry_hash = BundleReader::from_bytes(&base_bundle)
        .unwrap()
        .manifest()
        .entry(base_ids[0])
        .unwrap()
        .content_hash;
    let base_install = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                base_record_seed.bundle_id,
                "artifact_runtime_base",
                AssetIoLayerKind::BaseBundle,
                10,
                "base/runtime_base.bundle",
            ),
            &base_bundle,
        )
        .unwrap();

    let composite = store.build_composite_io(&registry).unwrap();
    let (metadata, resolution) = composite
        .metadata_with_diagnostics("textures/runtime_artifact.texture")
        .unwrap();
    assert_eq!(metadata.size, texture_bytes(1, 1, 15).len() as u64);
    assert_eq!(resolution.layer.name, "artifact_runtime_base");
    assert_eq!(resolution.layer.kind, AssetIoLayerKind::BaseBundle);

    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(composite);
    server.register_builtin_loaders();
    let activation = server
        .activate_asset_package_registry_from_artifacts(
            registry.clone(),
            AssetPackageUpdatePolicy::default(),
            &root,
        )
        .unwrap();
    let group = server.preload_bundle(&activation.mounted_bundles[0]);
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(51))),
    );
    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let handle = Handle::<Texture>::strong(base_ids[0]);
    assert!(server.is_ready(&handle));
    assert_eq!(
        server.metadata(base_ids[0]).unwrap().path,
        Some(AssetPath::parse("textures/runtime_artifact.texture"))
    );
    assert_eq!(
        server.metadata(base_ids[0]).unwrap().source_hash,
        Some(entry_hash)
    );

    let missing_patch = AssetPackageRecord::new(
        patch_record.bundle_id,
        "artifact_runtime_patch",
        AssetIoLayerKind::Patch,
        0,
        true,
        "patches/runtime_patch.bundle",
        patch_record.manifest.clone(),
    );
    let missing_registry =
        AssetPackageRegistry::new(vec![missing_patch.clone(), base_install.record.clone()])
            .unwrap();
    let missing_report = server
        .verify_asset_package_artifacts(&missing_registry, &root)
        .unwrap();
    assert!(!missing_report.all_available());
    assert!(missing_report
        .packages
        .iter()
        .any(|status| status.package == "artifact_runtime_patch"
            && status.message.as_deref() == Some("artifact file is missing")));
    assert!(matches!(
        server.activate_asset_package_registry_from_artifacts(
            missing_registry.clone(),
            AssetPackageUpdatePolicy::default(),
            &root,
        ),
        Err(AssetError::Bundle { message }) if message.contains("artifact_runtime_patch")
    ));
    assert!(server.mounted_bundle(patch_record.bundle_id).is_none());
    assert_eq!(server.asset_package_registry().packages().len(), 1);
    assert!(server.is_ready(&handle));

    let patch_artifact = store.artifact_path_for_record(&missing_patch).unwrap();
    std::fs::create_dir_all(patch_artifact.parent().unwrap()).unwrap();
    std::fs::write(&patch_artifact, &base_bundle).unwrap();
    let mismatch_report = store.verify_registry(&missing_registry).unwrap();
    let patch_status = mismatch_report
        .packages
        .iter()
        .find(|status| status.package == "artifact_runtime_patch")
        .unwrap();
    assert_eq!(patch_status.manifest_matches, Some(false));
    assert!(matches!(
        mismatch_report.require_available(),
        Err(AssetError::Bundle { message }) if message.contains("manifest")
    ));
    assert!(matches!(
        server.activate_asset_package_registry_from_artifacts(
            missing_registry,
            AssetPackageUpdatePolicy::default(),
            &root,
        ),
        Err(AssetError::Bundle { message }) if message.contains("manifest")
    ));
    assert!(server.mounted_bundle(patch_record.bundle_id).is_none());
    assert!(server.is_ready(&handle));

    std::fs::write(&patch_artifact, &patch_bundle).unwrap();
    assert!(store
        .verify_registry(
            &AssetPackageRegistry::new(vec![missing_patch, base_install.record]).unwrap()
        )
        .unwrap()
        .all_available());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn asset_package_update_policy_reports_version_changes_and_v1_compatibility() {
    let (current_base, _base_bundle, _) = texture_package(
        "base_versioned",
        AssetIoLayerKind::BaseBundle,
        10,
        BundleId(10),
        "packages/base_versioned.nga_bundle",
        vec![("textures/base_versioned.texture", texture_bytes(1, 1, 1))],
    );
    let current_base = current_base.with_package_version(2);
    let current = AssetPackageRegistry::new(vec![current_base.clone()]).unwrap();

    let downgraded_base = current_base.clone().with_package_version(1);
    let (future_patch, _future_bundle, _) = texture_package(
        "future_patch",
        AssetIoLayerKind::Patch,
        0,
        BundleId(11),
        "packages/future_patch.nga_bundle",
        vec![("textures/future.texture", texture_bytes(1, 1, 2))],
    );
    let future_patch = future_patch.with_minimum_runtime_version(2);
    let next = AssetPackageRegistry::new(vec![future_patch.clone(), downgraded_base]).unwrap();

    let report = current
        .update_report(&next, AssetPackageUpdatePolicy::default())
        .unwrap();
    assert!(!report.is_compatible());
    assert_eq!(report.added[0].name, "future_patch");
    assert_eq!(report.updated[0].name, "base_versioned");
    assert_eq!(
        report
            .compatibility_issues
            .iter()
            .map(|issue| issue.kind)
            .collect::<Vec<_>>(),
        vec![
            AssetPackageCompatibilityIssueKind::RuntimeTooOld,
            AssetPackageCompatibilityIssueKind::VersionDowngrade,
        ]
    );
    assert!(matches!(
        report.require_compatible(),
        Err(AssetError::Bundle { message }) if message.contains("runtime") || message.contains("downgrade")
    ));

    let compatible = current
        .update_report(
            &next,
            AssetPackageUpdatePolicy::new(2).with_version_downgrade_allowed(true),
        )
        .unwrap();
    assert!(compatible.is_compatible());

    let v2_text = current.to_text();
    let v2_lines = v2_text.lines().collect::<Vec<_>>();
    let v2_package_fields = v2_lines[2].split('|').collect::<Vec<_>>();
    let manifest_line_count = v2_package_fields[10];
    let mut v1_lines = vec![
        "NGA_ASSET_PACKAGE_REGISTRY_V1".to_owned(),
        "packages=1".to_owned(),
        format!(
            "package|{}|{}|{}|{}|{}|{}|{}",
            current_base.bundle_id.0,
            current_base.priority,
            current_base.enabled,
            "base_bundle",
            current_base.name,
            current_base.bundle_path,
            manifest_line_count
        ),
    ];
    v1_lines.extend(v2_lines[3..].iter().map(|line| (*line).to_owned()));
    let v1 = AssetPackageRegistry::from_text(&v1_lines.join("\n")).unwrap();
    assert_eq!(v1.packages()[0].package_version, 1);
    assert_eq!(v1.packages()[0].minimum_runtime_version, 1);
}

#[test]
fn asset_package_dependency_compatibility_reports_missing_and_version_bounds() {
    let (base, _base_bundle, _) = texture_package(
        "base_dependency",
        AssetIoLayerKind::BaseBundle,
        10,
        BundleId(30),
        "packages/base_dependency.bundle",
        vec![("textures/base_dependency.texture", texture_bytes(1, 1, 1))],
    );
    let base = base.with_package_version(2);
    let (toolkit, _toolkit_bundle, _) = texture_package(
        "toolkit_dependency",
        AssetIoLayerKind::BaseBundle,
        20,
        BundleId(31),
        "packages/toolkit_dependency.bundle",
        vec![(
            "textures/toolkit_dependency.texture",
            texture_bytes(1, 1, 2),
        )],
    );
    let toolkit = toolkit.with_package_version(5);
    let (patch, _patch_bundle, _) = texture_package(
        "patch_dependency",
        AssetIoLayerKind::Patch,
        0,
        BundleId(32),
        "packages/patch_dependency.bundle",
        vec![("textures/patch_dependency.texture", texture_bytes(1, 1, 3))],
    );
    let patch = patch
        .with_package_dependency(AssetPackageDependency::new("base_dependency", 3))
        .with_package_dependency(AssetPackageDependency::new("missing_dependency", 1))
        .with_package_dependency(
            AssetPackageDependency::new("toolkit_dependency", 1).with_max_version(4),
        );

    let current = AssetPackageRegistry::new(vec![base.clone(), toolkit.clone()]).unwrap();
    let next =
        AssetPackageRegistry::new(vec![patch.clone(), base.clone(), toolkit.clone()]).unwrap();
    assert!(next.to_text().starts_with("NGA_ASSET_PACKAGE_REGISTRY_V3"));
    let restored = AssetPackageRegistry::from_text(&next.to_text()).unwrap();
    assert_eq!(
        restored.packages()[0].package_dependencies,
        patch.package_dependencies
    );

    let report = current
        .update_report(&next, AssetPackageUpdatePolicy::default())
        .unwrap();
    assert!(!report.is_compatible());
    assert_eq!(
        report
            .compatibility_issues
            .iter()
            .map(|issue| (&issue.kind, issue.dependency.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            (
                &AssetPackageCompatibilityIssueKind::PackageDependencyTooOld,
                Some("base_dependency"),
            ),
            (
                &AssetPackageCompatibilityIssueKind::MissingPackageDependency,
                Some("missing_dependency"),
            ),
            (
                &AssetPackageCompatibilityIssueKind::PackageDependencyTooNew,
                Some("toolkit_dependency"),
            ),
        ]
    );
    let old_issue = &report.compatibility_issues[0];
    assert_eq!(old_issue.dependency_version, Some(2));
    assert_eq!(old_issue.required_min_version, Some(3));
    let new_issue = &report.compatibility_issues[2];
    assert_eq!(new_issue.dependency_version, Some(5));
    assert_eq!(new_issue.required_max_version, Some(4));
    assert!(matches!(
        report.require_compatible(),
        Err(AssetError::Bundle { message }) if message.contains("base_dependency")
    ));
    let mut server = AssetServer::new(AssetServerConfig::default());
    server
        .activate_asset_package_registry(current.clone(), AssetPackageUpdatePolicy::default())
        .unwrap();
    assert!(matches!(
        server.activate_asset_package_registry(next.clone(), AssetPackageUpdatePolicy::default()),
        Err(AssetError::Bundle { message }) if message.contains("base_dependency")
    ));
    assert_eq!(server.asset_package_registry(), &current);

    let satisfied_patch = patch.clone().with_package_dependencies(vec![
        AssetPackageDependency::new("base_dependency", 2),
        AssetPackageDependency::new("toolkit_dependency", 1).with_max_version(5),
    ]);
    let satisfied =
        AssetPackageRegistry::new(vec![satisfied_patch, base.with_package_version(3), toolkit])
            .unwrap();
    assert!(current
        .update_report(&satisfied, AssetPackageUpdatePolicy::default())
        .unwrap()
        .is_compatible());

    assert!(matches!(
        AssetPackageRegistry::new(vec![patch.clone().with_package_dependency(
            AssetPackageDependency::new("patch_dependency", 1)
        )]),
        Err(AssetError::Bundle { message }) if message.contains("cannot depend on itself")
    ));
    assert!(matches!(
        AssetPackageRegistry::new(vec![patch.clone().with_package_dependencies(vec![
            AssetPackageDependency::new("base_dependency", 4).with_max_version(3)
        ])]),
        Err(AssetError::Bundle { message }) if message.contains("max version is lower")
    ));
    assert!(matches!(
        AssetPackageRegistry::new(vec![patch.clone().with_package_dependency(
            AssetPackageDependency::new("base_dependency", 0)
        )]),
        Err(AssetError::Bundle { message }) if message.contains("min version must be greater than zero")
    ));
    assert!(matches!(
        AssetPackageRegistry::new(vec![patch.with_package_dependencies(vec![
            AssetPackageDependency::new("base_dependency", 2),
            AssetPackageDependency::new("base_dependency", 3),
        ])]),
        Err(AssetError::Bundle { message }) if message.contains("duplicate dependency")
    ));
}

#[test]
fn asset_package_dependency_compatibility_reports_separator_diagnostics() {
    let (package, _bundle, _) = texture_package(
        "separator_package",
        AssetIoLayerKind::BaseBundle,
        5,
        BundleId(33),
        "packages/separator_package.bundle",
        vec![("textures/separator_package.texture", texture_bytes(1, 1, 4))],
    );

    assert!(matches!(
        AssetPackageRegistry::new(vec![package.clone().with_package_dependency(
            AssetPackageDependency::new("bad:dependency", 1)
        )]),
        Err(AssetError::Bundle { message }) if message.contains("dependency separators")
    ));
    assert!(matches!(
        AssetPackageRegistry::new(vec![package.with_package_dependency(
            AssetPackageDependency::new("bad,dependency", 1)
        )]),
        Err(AssetError::Bundle { message }) if message.contains("dependency separators")
    ));
}

#[test]
fn mounted_bundle_registry_reports_manifest_line_count_overflow() {
    let registry = MountedBundleRegistry::new(vec![MountedBundle {
        id: BundleId(1),
        name: "overflow".to_owned(),
        manifest: BundleManifest {
            name: "overflow".to_owned(),
            compression: CompressionKind::None,
            chunks: Vec::new(),
            entries: Vec::new(),
        },
    }]);
    let overflow_text =
        registry
            .to_text()
            .replacen("bundle|1|5", &format!("bundle|1|{}", usize::MAX), 1);
    assert!(matches!(
        MountedBundleRegistry::from_text(&overflow_text),
        Err(AssetError::Bundle { message }) if message.contains("mounted bundle manifest line count overflow")
    ));
}

#[test]
fn mounted_bundle_registry_reports_missing_and_invalid_bundle_count_lines() {
    assert!(matches!(
        MountedBundleRegistry::from_text("NGA_MOUNTED_BUNDLE_REGISTRY_V1"),
        Err(AssetError::Bundle { message }) if message.contains("missing `bundles=` line")
    ));
    assert!(matches!(
        MountedBundleRegistry::from_text("NGA_MOUNTED_BUNDLE_REGISTRY_V1\nbundle=1"),
        Err(AssetError::Bundle { message }) if message.contains("expected `bundles=` line")
    ));
    assert!(matches!(
        MountedBundleRegistry::from_text("NGA_MOUNTED_BUNDLE_REGISTRY_V1\nbundles=abc"),
        Err(AssetError::Bundle { message }) if message.contains("invalid mounted bundle count")
    ));
}

#[test]
fn mounted_bundle_registry_reports_missing_and_invalid_bundle_lines() {
    assert!(matches!(
        MountedBundleRegistry::from_text("NGA_MOUNTED_BUNDLE_REGISTRY_V1\nbundles=1"),
        Err(AssetError::Bundle { message }) if message.contains("missing mounted bundle line 0")
    ));
    assert!(matches!(
        MountedBundleRegistry::from_text("NGA_MOUNTED_BUNDLE_REGISTRY_V1\nbundles=1\nbundlex|1|1"),
        Err(AssetError::Bundle { message }) if message.contains("invalid mounted bundle line 0")
    ));
    assert!(matches!(
        MountedBundleRegistry::from_text("NGA_MOUNTED_BUNDLE_REGISTRY_V1\nbundles=1\nbundle|abc|1"),
        Err(AssetError::Bundle { message }) if message.contains("invalid mounted bundle id")
    ));
    assert!(matches!(
        MountedBundleRegistry::from_text("NGA_MOUNTED_BUNDLE_REGISTRY_V1\nbundles=1\nbundle|1|2\nNGA_MOUNTED_BUNDLE_REGISTRY_V1"),
        Err(AssetError::Bundle { message }) if message.contains("mounted bundle 0 manifest is truncated")
    ));
    assert!(matches!(
        MountedBundleRegistry::from_text(
            "NGA_MOUNTED_BUNDLE_REGISTRY_V1\nbundles=1\nbundle|1|5\nNGA_BUNDLE_V2\nname=patch\ncompression=none\nchunks=0\nentries=0\nextra"
        ),
        Err(AssetError::Bundle { message }) if message.contains("unexpected trailing mounted bundle registry data")
    ));
    assert!(matches!(
        MountedBundleRegistry::from_text(
            "NGA_MOUNTED_BUNDLE_REGISTRY_V1\nbundles=1\nbundle|1|6\nNGA_BUNDLE_V2\nname=patch\ncompression=none\nchunks=0\nentries=0\nextra"
        ),
        Err(AssetError::Bundle { message }) if message.contains("unexpected trailing bundle manifest data")
    ));
}

#[test]
fn asset_package_asset_override_report_tracks_semantic_policy_issues() {
    let base_dependency = AssetId::new();
    let base_material = AssetId::new();
    let patch_dependency = AssetId::new();
    let patch_material = AssetId::new();
    let material_path = AssetPath::parse("materials/hero.material");
    let base_material_bytes = b"shader=shaders/base.shader\ntexture.albedo=textures/base.texture\n";
    let patch_material_bytes =
        b"shader=shaders/patch.shader\ntexture.albedo=textures/patch.texture\n";

    let (base, _base_bundle) = package_from_assets(
        "semantic_base",
        AssetIoLayerKind::BaseBundle,
        20,
        BundleId(40),
        "packages/semantic_base.bundle",
        vec![
            BundleAsset {
                id: base_dependency,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/base.texture"),
                bytes: texture_bytes(1, 1, 4),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: base_material,
                asset_type: AssetTypeId::of::<Material>(),
                path: material_path.clone(),
                bytes: base_material_bytes.to_vec(),
                dependencies: vec![base_dependency],
            },
        ],
    );
    let (patch, _patch_bundle) = package_from_assets(
        "semantic_patch",
        AssetIoLayerKind::Patch,
        0,
        BundleId(41),
        "packages/semantic_patch.bundle",
        vec![
            BundleAsset {
                id: patch_dependency,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/patch.texture"),
                bytes: texture_bytes(1, 1, 8),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: patch_material,
                asset_type: AssetTypeId::of::<Material>(),
                path: material_path.clone(),
                bytes: patch_material_bytes.to_vec(),
                dependencies: vec![patch_dependency],
            },
        ],
    );
    let current = AssetPackageRegistry::new(vec![base.clone()]).unwrap();
    let next = AssetPackageRegistry::new(vec![patch.clone(), base.clone()]).unwrap();

    let override_report = next.asset_override_report();
    assert!(override_report.has_overrides());
    assert!(override_report.has_issues());
    assert_eq!(override_report.overrides.len(), 1);
    let asset_override = &override_report.overrides[0];
    assert_eq!(asset_override.path, material_path);
    assert_eq!(asset_override.winner.name, "semantic_patch");
    assert_eq!(asset_override.shadowed.name, "semantic_base");
    assert_eq!(asset_override.winner_asset.id, patch_material);
    assert_eq!(asset_override.shadowed_asset.id, base_material);
    assert_eq!(
        asset_override.winner_asset.asset_type,
        AssetTypeId::of::<Material>()
    );
    assert_eq!(
        asset_override.shadowed_asset.asset_type,
        AssetTypeId::of::<Material>()
    );
    assert_eq!(
        asset_override.issues,
        vec![
            AssetPackageAssetOverrideIssueKind::AssetIdChanged,
            AssetPackageAssetOverrideIssueKind::ContentHashChanged,
            AssetPackageAssetOverrideIssueKind::DependenciesChanged,
            AssetPackageAssetOverrideIssueKind::DependencyProvidersChanged,
        ]
    );
    assert_eq!(
        asset_override.winner_dependency_providers[0]
            .provider
            .as_ref()
            .map(|provider| provider.name.as_str()),
        Some("semantic_patch")
    );
    assert_eq!(
        asset_override.shadowed_dependency_providers[0]
            .provider
            .as_ref()
            .map(|provider| provider.name.as_str()),
        Some("semantic_base")
    );

    let default_report = current
        .update_report(&next, AssetPackageUpdatePolicy::default())
        .unwrap();
    assert!(default_report.is_compatible());
    assert_eq!(default_report.asset_overrides, override_report);

    let strict_policy = AssetPackageUpdatePolicy::default()
        .with_asset_compatibility(AssetPackageAssetCompatibilityPolicy::strict());
    let strict_report = current.update_report(&next, strict_policy).unwrap();
    assert!(!strict_report.is_compatible());
    assert_eq!(
        strict_report
            .compatibility_issues
            .iter()
            .map(|issue| issue.kind)
            .collect::<Vec<_>>(),
        vec![
            AssetPackageCompatibilityIssueKind::AssetIdChanged,
            AssetPackageCompatibilityIssueKind::AssetContentHashChanged,
            AssetPackageCompatibilityIssueKind::AssetDependenciesChanged,
            AssetPackageCompatibilityIssueKind::AssetDependencyProvidersChanged,
        ]
    );
    assert_eq!(
        strict_report.compatibility_issues[0]
            .asset_override
            .as_ref()
            .unwrap()
            .winner_asset
            .id,
        patch_material
    );
    assert!(matches!(
        strict_report.require_compatible(),
        Err(AssetError::Bundle { message })
            if message.contains("semantic_patch") && message.contains("asset id")
    ));

    let mut server = AssetServer::new(AssetServerConfig::default());
    server
        .activate_asset_package_registry(current.clone(), AssetPackageUpdatePolicy::default())
        .unwrap();
    assert!(matches!(
        server.activate_asset_package_registry(next.clone(), strict_policy),
        Err(AssetError::Bundle { message })
            if message.contains("semantic_patch") && message.contains("asset id")
    ));
    assert_eq!(server.asset_package_registry(), &current);

    let wrong_type_id = AssetId::new();
    let (wrong_type_patch, _wrong_type_bundle) = package_from_assets(
        "semantic_wrong_type_patch",
        AssetIoLayerKind::Patch,
        0,
        BundleId(42),
        "packages/semantic_wrong_type_patch.bundle",
        vec![BundleAsset {
            id: wrong_type_id,
            asset_type: AssetTypeId::of::<Shader>(),
            path: AssetPath::parse("materials/hero.material"),
            bytes: b"#vertex\n".to_vec(),
            dependencies: Vec::new(),
        }],
    );
    let wrong_type_next = AssetPackageRegistry::new(vec![wrong_type_patch, base.clone()]).unwrap();
    let wrong_type_report = current
        .update_report(&wrong_type_next, AssetPackageUpdatePolicy::default())
        .unwrap();
    assert!(!wrong_type_report.is_compatible());
    assert_eq!(
        wrong_type_report.compatibility_issues[0].kind,
        AssetPackageCompatibilityIssueKind::AssetTypeChanged
    );
    assert!(matches!(
        server.activate_asset_package_registry(
            wrong_type_next,
            AssetPackageUpdatePolicy::default(),
        ),
        Err(AssetError::Bundle { message })
            if message.contains("semantic_wrong_type_patch")
                && message.contains("asset type")
    ));
}

#[test]
fn asset_package_asset_override_report_tracks_scene_and_prefab_semantic_policy_issues() {
    let base_scene_dependency = AssetId::new();
    let base_scene = AssetId::new();
    let base_prefab_dependency = AssetId::new();
    let base_prefab = AssetId::new();
    let patch_scene_dependency = AssetId::new();
    let patch_scene = AssetId::new();
    let patch_prefab_dependency = AssetId::new();
    let patch_prefab = AssetId::new();
    let scene_path = AssetPath::parse("scenes/arena.scene");
    let prefab_path = AssetPath::parse("prefabs/hero.prefab");

    let (base, _base_bundle) = package_from_assets(
        "semantic_scene_base",
        AssetIoLayerKind::BaseBundle,
        20,
        BundleId(42),
        "packages/semantic_scene_base.bundle",
        vec![
            BundleAsset {
                id: base_scene_dependency,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/base.scene.texture"),
                bytes: texture_bytes(1, 1, 4),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: base_scene,
                asset_type: AssetTypeId::of::<SceneAsset>(),
                path: scene_path.clone(),
                bytes: scene_bytes("arena", "textures/base.scene.texture"),
                dependencies: vec![base_scene_dependency],
            },
            BundleAsset {
                id: base_prefab_dependency,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/base.prefab.texture"),
                bytes: texture_bytes(1, 1, 6),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: base_prefab,
                asset_type: AssetTypeId::of::<Prefab>(),
                path: prefab_path.clone(),
                bytes: prefab_bytes("hero", "textures/base.prefab.texture"),
                dependencies: vec![base_prefab_dependency],
            },
        ],
    );
    let (patch, _patch_bundle) = package_from_assets(
        "semantic_scene_patch",
        AssetIoLayerKind::Patch,
        0,
        BundleId(43),
        "packages/semantic_scene_patch.bundle",
        vec![
            BundleAsset {
                id: patch_scene_dependency,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/patch.scene.texture"),
                bytes: texture_bytes(1, 1, 8),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: patch_scene,
                asset_type: AssetTypeId::of::<SceneAsset>(),
                path: scene_path.clone(),
                bytes: scene_bytes("arena", "textures/patch.scene.texture"),
                dependencies: vec![patch_scene_dependency],
            },
            BundleAsset {
                id: patch_prefab_dependency,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/patch.prefab.texture"),
                bytes: texture_bytes(1, 1, 10),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: patch_prefab,
                asset_type: AssetTypeId::of::<Prefab>(),
                path: prefab_path.clone(),
                bytes: prefab_bytes("hero", "textures/patch.prefab.texture"),
                dependencies: vec![patch_prefab_dependency],
            },
        ],
    );
    let current = AssetPackageRegistry::new(vec![base.clone()]).unwrap();
    let next = AssetPackageRegistry::new(vec![patch.clone(), base.clone()]).unwrap();

    let override_report = next.asset_override_report();
    assert!(override_report.has_overrides());
    assert!(override_report.has_issues());
    assert_eq!(override_report.overrides.len(), 2);

    let scene_override = override_report
        .overrides
        .iter()
        .find(|asset_override| asset_override.path == scene_path)
        .unwrap();
    assert_eq!(scene_override.winner.name, "semantic_scene_patch");
    assert_eq!(scene_override.shadowed.name, "semantic_scene_base");
    assert_eq!(scene_override.winner_asset.id, patch_scene);
    assert_eq!(scene_override.shadowed_asset.id, base_scene);
    assert_eq!(
        scene_override.winner_asset.asset_type,
        AssetTypeId::of::<SceneAsset>()
    );
    assert_eq!(
        scene_override.shadowed_asset.asset_type,
        AssetTypeId::of::<SceneAsset>()
    );
    assert_eq!(
        scene_override.issues,
        vec![
            AssetPackageAssetOverrideIssueKind::AssetIdChanged,
            AssetPackageAssetOverrideIssueKind::ContentHashChanged,
            AssetPackageAssetOverrideIssueKind::DependenciesChanged,
            AssetPackageAssetOverrideIssueKind::DependencyProvidersChanged,
        ]
    );
    assert_eq!(
        scene_override.winner_dependency_providers[0]
            .provider
            .as_ref()
            .map(|provider| provider.name.as_str()),
        Some("semantic_scene_patch")
    );
    assert_eq!(
        scene_override.shadowed_dependency_providers[0]
            .provider
            .as_ref()
            .map(|provider| provider.name.as_str()),
        Some("semantic_scene_base")
    );

    let prefab_override = override_report
        .overrides
        .iter()
        .find(|asset_override| asset_override.path == prefab_path)
        .unwrap();
    assert_eq!(prefab_override.winner.name, "semantic_scene_patch");
    assert_eq!(prefab_override.shadowed.name, "semantic_scene_base");
    assert_eq!(prefab_override.winner_asset.id, patch_prefab);
    assert_eq!(prefab_override.shadowed_asset.id, base_prefab);
    assert_eq!(
        prefab_override.winner_asset.asset_type,
        AssetTypeId::of::<Prefab>()
    );
    assert_eq!(
        prefab_override.shadowed_asset.asset_type,
        AssetTypeId::of::<Prefab>()
    );
    assert_eq!(
        prefab_override.issues,
        vec![
            AssetPackageAssetOverrideIssueKind::AssetIdChanged,
            AssetPackageAssetOverrideIssueKind::ContentHashChanged,
            AssetPackageAssetOverrideIssueKind::DependenciesChanged,
            AssetPackageAssetOverrideIssueKind::DependencyProvidersChanged,
        ]
    );
    assert_eq!(
        prefab_override.winner_dependency_providers[0]
            .provider
            .as_ref()
            .map(|provider| provider.name.as_str()),
        Some("semantic_scene_patch")
    );
    assert_eq!(
        prefab_override.shadowed_dependency_providers[0]
            .provider
            .as_ref()
            .map(|provider| provider.name.as_str()),
        Some("semantic_scene_base")
    );

    let default_report = current
        .update_report(&next, AssetPackageUpdatePolicy::default())
        .unwrap();
    assert!(default_report.is_compatible());
    assert_eq!(default_report.asset_overrides, override_report);

    let strict_policy = AssetPackageUpdatePolicy::default()
        .with_asset_compatibility(AssetPackageAssetCompatibilityPolicy::strict());
    let strict_report = current.update_report(&next, strict_policy).unwrap();
    assert!(!strict_report.is_compatible());
    assert_eq!(strict_report.compatibility_issues.len(), 8);
    assert_eq!(
        strict_report
            .compatibility_issues
            .iter()
            .filter(|issue| {
                issue
                    .asset_override
                    .as_ref()
                    .map(|asset_override| asset_override.path == scene_path)
                    .unwrap_or(false)
            })
            .count(),
        4
    );
    assert_eq!(
        strict_report
            .compatibility_issues
            .iter()
            .filter(|issue| {
                issue
                    .asset_override
                    .as_ref()
                    .map(|asset_override| asset_override.path == prefab_path)
                    .unwrap_or(false)
            })
            .count(),
        4
    );
    let mut server = AssetServer::new(AssetServerConfig::default());
    server
        .activate_asset_package_registry(current.clone(), AssetPackageUpdatePolicy::default())
        .unwrap();
    assert!(matches!(
        server.activate_asset_package_registry(next.clone(), strict_policy),
        Err(AssetError::Bundle { message })
            if message.contains("semantic_scene_patch") && message.contains("asset id")
    ));
    assert_eq!(server.asset_package_registry(), &current);
}

#[test]
fn asset_server_rejects_incompatible_package_activation_transactionally() {
    let (base, base_bundle, base_ids) = texture_package(
        "base_transaction",
        AssetIoLayerKind::BaseBundle,
        10,
        BundleId(10),
        "packages/base_transaction.nga_bundle",
        vec![("textures/transaction.texture", texture_bytes(1, 1, 7))],
    );
    let base = base.with_package_version(2);
    let (patch, _patch_bundle, _) = texture_package(
        "patch_transaction",
        AssetIoLayerKind::Patch,
        0,
        BundleId(11),
        "packages/patch_transaction.nga_bundle",
        vec![("textures/transaction.texture", texture_bytes(1, 1, 9))],
    );

    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(BundleAssetIo::from_bytes(&base_bundle).unwrap());
    server.register_builtin_loaders();
    let current = AssetPackageRegistry::new(vec![base.clone()]).unwrap();
    let mounted = server
        .activate_asset_package_registry(current, AssetPackageUpdatePolicy::default())
        .unwrap();
    let group = server.preload_bundle(&mounted.mounted_bundles[0]);
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(41))),
    );
    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let handle = Handle::<Texture>::strong(base_ids[0]);
    assert!(server.is_ready(&handle));

    let downgraded = base.clone().with_package_version(1);
    let invalid = AssetPackageRegistry::new(vec![downgraded, patch.clone()]).unwrap();
    let preview = server
        .preview_asset_package_update(&invalid, AssetPackageUpdatePolicy::default())
        .unwrap();
    assert!(!preview.is_compatible());
    assert!(matches!(
        server.activate_asset_package_registry(invalid, AssetPackageUpdatePolicy::default()),
        Err(AssetError::Bundle { message }) if message.contains("incompatible")
    ));
    assert_eq!(
        server.asset_package_registry().packages()[0].name,
        "base_transaction"
    );
    assert_eq!(
        server.asset_package_registry().packages()[0].package_version,
        2
    );
    assert!(server.mounted_bundle(patch.bundle_id).is_none());
    assert_eq!(server.state_by_id(base_ids[0]), AssetLoadState::Ready);
    assert!(server.is_ready(&handle));

    let valid = AssetPackageRegistry::new(vec![patch.clone(), base.clone()]).unwrap();
    let activation = server
        .activate_asset_package_registry(valid, AssetPackageUpdatePolicy::default())
        .unwrap();
    assert!(activation.report.is_compatible());
    assert!(activation.report.conflicts.has_conflicts());
    assert_eq!(activation.mounted_bundles.len(), 2);
    assert!(server.mounted_bundle(patch.bundle_id).is_some());
    assert_eq!(server.state_by_id(base_ids[0]), AssetLoadState::Ready);
}

#[test]
fn asset_server_rejects_packages_for_runtime_version_too_old() {
    let (base, base_bundle, _base_ids) = texture_package(
        "base_runtime_gate",
        AssetIoLayerKind::BaseBundle,
        10,
        BundleId(12),
        "packages/base_runtime_gate.nga_bundle",
        vec![("textures/runtime_gate.texture", texture_bytes(1, 1, 6))],
    );
    let gated = base.with_minimum_runtime_version(2);

    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(BundleAssetIo::from_bytes(&base_bundle).unwrap());
    server.register_builtin_loaders();

    let registry = AssetPackageRegistry::new(vec![gated.clone()]).unwrap();
    let preview = server
        .preview_asset_package_update(&registry, AssetPackageUpdatePolicy::default())
        .unwrap();
    assert!(!preview.is_compatible());
    assert_eq!(
        preview.compatibility_issues[0].kind,
        AssetPackageCompatibilityIssueKind::RuntimeTooOld
    );
    assert_eq!(preview.compatibility_issues[0].runtime_version, 1);
    assert_eq!(preview.compatibility_issues[0].minimum_runtime_version, 2);
    assert!(matches!(
        server.activate_asset_package_registry(registry, AssetPackageUpdatePolicy::default()),
        Err(AssetError::Bundle { message })
            if message.contains("base_runtime_gate") && message.contains("requires runtime package version 2, current runtime is 1")
    ));
    assert!(server.asset_package_registry().packages().is_empty());

    let upgraded_policy = AssetPackageUpdatePolicy::new(2);
    let activation = server
        .activate_asset_package_registry(
            AssetPackageRegistry::new(vec![gated]).unwrap(),
            upgraded_policy,
        )
        .unwrap();
    assert!(activation.report.is_compatible());
    assert_eq!(activation.mounted_bundles.len(), 1);
    assert_eq!(server.asset_package_registry().packages().len(), 1);
}

#[test]
fn asset_server_restores_package_registry_without_disrupting_ready_assets() {
    let ready_bytes = texture_bytes(1, 1, 7);
    let (base, base_bundle, base_ids) = texture_package(
        "base_runtime",
        AssetIoLayerKind::BaseBundle,
        10,
        BundleId(10),
        "packages/base_runtime.nga_bundle",
        vec![("textures/runtime.texture", ready_bytes.clone())],
    );
    let (patch, _patch_bundle, _) = texture_package(
        "patch_runtime",
        AssetIoLayerKind::Patch,
        0,
        BundleId(11),
        "packages/patch_runtime.nga_bundle",
        vec![("textures/runtime.texture", texture_bytes(1, 1, 9))],
    );
    let entry_hash = BundleReader::from_bytes(&base_bundle)
        .unwrap()
        .manifest()
        .entry(base_ids[0])
        .unwrap()
        .content_hash;
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(BundleAssetIo::from_bytes(&base_bundle).unwrap());
    server.register_builtin_loaders();
    let initial = server
        .restore_asset_package_registry(AssetPackageRegistry::new(vec![base.clone()]).unwrap())
        .unwrap();
    let group = server.preload_bundle(&initial[0]);
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(31))),
    );
    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    let handle = Handle::<Texture>::strong(base_ids[0]);
    assert!(server.is_ready(&handle));
    assert_eq!(server.state_by_id(base_ids[0]), AssetLoadState::Ready);
    assert_eq!(
        server.metadata(base_ids[0]).unwrap().source_hash,
        Some(entry_hash)
    );

    let registry = AssetPackageRegistry::new(vec![patch.clone(), base.clone()]).unwrap();
    let mounted = server.restore_asset_package_registry(registry).unwrap();
    assert_eq!(mounted.len(), 2);
    assert_eq!(server.asset_package_registry().packages().len(), 2);
    assert!(server.mounted_bundle(patch.bundle_id).is_some());
    assert!(server.mounted_bundle(base.bundle_id).is_some());
    assert_eq!(server.state_by_id(base_ids[0]), AssetLoadState::Ready);
    assert_eq!(server.get(&handle).unwrap().width, 1);
    assert_eq!(
        server.metadata(base_ids[0]).unwrap().source_hash,
        Some(entry_hash)
    );

    let mut disabled_patch = patch.clone();
    disabled_patch.enabled = false;
    let reloaded = AssetPackageRegistry::new(vec![disabled_patch, base.clone()]).unwrap();
    let remounted = server.restore_asset_package_registry(reloaded).unwrap();
    assert_eq!(remounted.len(), 1);
    assert!(server.mounted_bundle(patch.bundle_id).is_none());
    assert!(server.mounted_bundle(base.bundle_id).is_some());
    assert_eq!(server.state_by_id(base_ids[0]), AssetLoadState::Ready);
    assert_eq!(
        server.metadata(base_ids[0]).unwrap().source_hash,
        Some(entry_hash)
    );
}

#[test]
fn asset_server_activates_artifact_registry_without_disrupting_ready_assets() {
    let root = temp_dir("package_artifact_reactivate");
    let _ = std::fs::remove_dir_all(&root);
    let store = AssetPackageArtifactStore::new(&root);
    let mut registry = AssetPackageRegistry::default();
    let (base_record_seed, base_bundle, base_ids) = texture_package(
        "artifact_react_base",
        AssetIoLayerKind::BaseBundle,
        10,
        BundleId(80),
        "unused/react_base.bundle",
        vec![("textures/react.texture", texture_bytes(1, 1, 15))],
    );
    let (patch_record_seed, patch_bundle, _) = texture_package(
        "artifact_react_patch",
        AssetIoLayerKind::Patch,
        0,
        BundleId(81),
        "unused/react_patch.bundle",
        vec![("textures/react.texture", texture_bytes(1, 1, 25))],
    );

    let base_install = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                base_record_seed.bundle_id,
                "artifact_react_base",
                AssetIoLayerKind::BaseBundle,
                10,
                "base/react_base.bundle",
            )
            .with_package_version(2),
            &base_bundle,
        )
        .unwrap();
    let patch_install = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                patch_record_seed.bundle_id,
                "artifact_react_patch",
                AssetIoLayerKind::Patch,
                0,
                "patches/react_patch.bundle",
            ),
            &patch_bundle,
        )
        .unwrap();

    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(store.build_composite_io(&registry).unwrap());
    server.register_builtin_loaders();
    let activation = server
        .activate_asset_package_registry_from_artifacts(
            registry.clone(),
            AssetPackageUpdatePolicy::default(),
            &root,
        )
        .unwrap();
    assert_eq!(activation.mounted_bundles.len(), 2);
    let groups = activation
        .mounted_bundles
        .iter()
        .map(|mounted| server.preload_bundle(mounted))
        .collect::<Vec<_>>();
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(71))),
    );
    assert!(groups
        .iter()
        .all(|group| server.group_state(group) == AssetLoadState::Ready));
    let handle = Handle::<Texture>::strong(base_ids[0]);
    assert!(server.is_ready(&handle));
    assert_eq!(
        server.metadata(base_ids[0]).unwrap().path,
        Some(AssetPath::parse("textures/react.texture"))
    );

    let mut disabled_patch = patch_install.record.clone();
    disabled_patch.enabled = false;
    let disabled_registry =
        AssetPackageRegistry::new(vec![disabled_patch, base_install.record.clone()]).unwrap();
    let restored = server
        .restore_asset_package_registry(disabled_registry.clone())
        .unwrap();
    assert_eq!(restored.len(), 1);
    assert!(server
        .mounted_bundle(patch_install.record.bundle_id)
        .is_none());
    assert!(server
        .mounted_bundle(base_install.record.bundle_id)
        .is_some());
    assert_eq!(server.state_by_id(base_ids[0]), AssetLoadState::Ready);
    assert!(server.is_ready(&handle));
    assert_eq!(
        server.metadata(base_ids[0]).unwrap().path,
        Some(AssetPath::parse("textures/react.texture"))
    );

    let activation = server
        .activate_asset_package_registry_from_artifacts(
            disabled_registry,
            AssetPackageUpdatePolicy::default(),
            &root,
        )
        .unwrap();
    assert_eq!(activation.mounted_bundles.len(), 1);
    assert_eq!(server.state_by_id(base_ids[0]), AssetLoadState::Ready);
    assert!(server.is_ready(&handle));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[cfg(feature = "zstd")]
fn asset_server_activates_zstd_artifact_registry_without_disrupting_ready_assets() {
    let root = temp_dir("package_artifact_reactivate_zstd");
    let _ = std::fs::remove_dir_all(&root);
    let store = AssetPackageArtifactStore::new(&root);
    let mut registry = AssetPackageRegistry::default();
    let (base_record_seed, base_bundle, base_ids) = texture_package(
        "artifact_zstd_base",
        AssetIoLayerKind::BaseBundle,
        10,
        BundleId(82),
        "unused/zstd_base.bundle",
        vec![("textures/zstd_react.texture", texture_bytes(1, 1, 35))],
    );
    let base_entry_hash = BundleReader::from_bytes(&base_bundle)
        .unwrap()
        .manifest()
        .entry(base_ids[0])
        .unwrap()
        .content_hash;
    let zstd_bundle = BundleWriter::build_bytes_with_options(
        "artifact_zstd_patch",
        BundleBuildOptions::new(CompressionKind::Zstd)
            .with_chunk_policy(BundleChunkPartitionPolicy::MaxUncompressedBytes(1)),
        vec![
            BundleAsset {
                id: AssetId::new(),
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/zstd_extra_a.texture"),
                bytes: texture_bytes(1, 1, 36),
                dependencies: Vec::new(),
            },
            BundleAsset {
                id: AssetId::new(),
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse("textures/zstd_extra_b.texture"),
                bytes: texture_bytes(1, 1, 37),
                dependencies: Vec::new(),
            },
        ],
    )
    .unwrap();
    let zstd_entry_hash = BundleReader::from_bytes(&zstd_bundle)
        .unwrap()
        .manifest()
        .entries
        .first()
        .unwrap()
        .content_hash;

    let _base_install = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                base_record_seed.bundle_id,
                "artifact_zstd_base",
                AssetIoLayerKind::BaseBundle,
                10,
                "base/zstd_base.bundle",
            )
            .with_package_version(2),
            &base_bundle,
        )
        .unwrap();
    let zstd_install = store
        .install_package_bytes(
            &mut registry,
            AssetPackageInstallRequest::new(
                BundleId(83),
                "artifact_zstd_patch",
                AssetIoLayerKind::Patch,
                1,
                "patches/zstd_patch.bundle",
            ),
            &zstd_bundle,
        )
        .unwrap();
    assert!(zstd_install.artifact_path.exists());
    assert_eq!(zstd_install.payload_size, zstd_bundle.len() as u64);
    assert_eq!(zstd_install.payload_hash, content_hash(&zstd_bundle));
    let composite = store.build_composite_io(&registry).unwrap();
    assert_eq!(
        composite.read("textures/zstd_extra_a.texture").unwrap(),
        texture_bytes(1, 1, 36)
    );
    assert_eq!(
        composite
            .metadata("textures/zstd_extra_b.texture")
            .unwrap()
            .hash,
        Some(content_hash(&texture_bytes(1, 1, 37)))
    );

    let registry_path = root.join("zstd_packages.txt");
    registry.save_to_file(&registry_path).unwrap();
    let loaded_registry = AssetPackageRegistry::load_from_file(&registry_path).unwrap();
    assert_eq!(loaded_registry, registry);
    let loaded_composite = store.build_composite_io(&loaded_registry).unwrap();
    assert_eq!(
        loaded_composite
            .read("textures/zstd_extra_a.texture")
            .unwrap(),
        texture_bytes(1, 1, 36)
    );
    assert_eq!(
        loaded_composite
            .metadata("textures/zstd_extra_b.texture")
            .unwrap()
            .hash,
        Some(content_hash(&texture_bytes(1, 1, 37)))
    );

    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(composite);
    server.register_builtin_loaders();
    let activation = server
        .activate_asset_package_registry_from_artifacts(
            registry.clone(),
            AssetPackageUpdatePolicy::default(),
            &root,
        )
        .unwrap();
    assert_eq!(activation.mounted_bundles.len(), 2);
    let groups = activation
        .mounted_bundles
        .iter()
        .map(|mounted| server.preload_bundle(mounted))
        .collect::<Vec<_>>();
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(72))),
    );
    assert!(groups
        .iter()
        .all(|group| server.group_state(group) == AssetLoadState::Ready));
    let base_handle = Handle::<Texture>::strong(base_ids[0]);
    assert!(server.is_ready(&base_handle));
    assert_eq!(
        server.metadata(base_ids[0]).unwrap().path,
        Some(AssetPath::parse("textures/zstd_react.texture"))
    );
    assert_eq!(
        server.metadata(base_ids[0]).unwrap().source_hash,
        Some(base_entry_hash)
    );
    let zstd_handle = server
        .asset_package_registry()
        .packages()
        .iter()
        .find(|package| package.name == "artifact_zstd_patch")
        .and_then(|package| package.manifest.entries.first())
        .map(|entry| Handle::<Texture>::strong(entry.id))
        .unwrap();
    assert!(server.is_ready(&zstd_handle));
    assert_eq!(server.state_by_id(zstd_handle.id()), AssetLoadState::Ready);
    assert_eq!(
        server.metadata(zstd_handle.id()).unwrap().source_hash,
        Some(zstd_entry_hash)
    );

    let removed = store
        .remove_package(&mut registry, "artifact_zstd_patch", true)
        .unwrap();
    assert_eq!(removed.removed.name, "artifact_zstd_patch");
    assert!(removed.artifact_removed);
    assert!(!removed.artifact_path.exists());
    assert!(store.verify_registry(&registry).unwrap().all_available());
    let restored = server
        .restore_asset_package_registry(registry.clone())
        .unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(server.asset_package_registry().packages().len(), 1);
    assert!(server.mounted_bundle(base_record_seed.bundle_id).is_some());
    assert!(server.mounted_bundle(BundleId(83)).is_none());
    assert!(server.is_ready(&base_handle));
    assert_eq!(
        server.metadata(base_ids[0]).unwrap().source_hash,
        Some(base_entry_hash)
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn asset_server_loads_texture_from_bundle_io() {
    let (id, bundle) = texture_bundle("textures/albedo.texture", texture_bytes(2, 1, 5));
    let bundle_io = BundleAssetIo::from_bytes(&bundle).unwrap();
    let entry_hash = bundle_io.manifest().entry(id).unwrap().content_hash;
    assert_eq!(
        bundle_io
            .manifest()
            .entry_by_path(&AssetPath::parse("textures/albedo.texture"))
            .unwrap()
            .id,
        id
    );

    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let texture: Handle<Texture> = server.load("textures/albedo.texture");
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(1))),
    );

    assert!(server.is_ready(&texture));
    assert_eq!(server.get(&texture).unwrap().width, 2);
    assert_eq!(
        server.metadata(texture.id()).unwrap().source_hash,
        Some(entry_hash)
    );
}

#[test]
fn asset_server_mounts_preloads_and_unmounts_bundle_manifest() {
    let (id, bundle) = texture_bundle("textures/preload.texture", texture_bytes(1, 2, 8));
    let bundle_io = BundleAssetIo::from_bytes(&bundle).unwrap();
    let entry_hash = bundle_io.manifest().entry(id).unwrap().content_hash;

    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle).unwrap();
    assert_eq!(server.mounted_bundle(mounted.id).unwrap().name, "textures");

    let group = server.preload_bundle(&mounted);
    assert_eq!(group.assets.len(), 1);
    assert_eq!(group.assets[0].id(), id);
    assert_eq!(
        server.path_from_id(id),
        Some(&AssetPath::parse("textures/preload.texture"))
    );

    server.update_loading();
    assert_eq!(server.state_by_id(id), AssetLoadState::UploadingGpu);
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(11))),
    );
    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(server.metadata(id).unwrap().source_hash, Some(entry_hash));

    let removed = server.unmount_bundle(mounted.id).unwrap();
    assert_eq!(removed.id, mounted.id);
    assert!(server.mounted_bundle(mounted.id).is_none());
    assert_eq!(server.state_by_id(id), AssetLoadState::Ready);
    assert_eq!(server.metadata(id).unwrap().source_hash, Some(entry_hash));
}

#[test]
fn mounted_bundle_registry_round_trip_preserves_metadata_and_can_remount() {
    let path = temp_file("mounted_bundle_registry", "txt");
    let _ = std::fs::remove_file(&path);
    let shader_id = AssetId::new();
    let texture_id = AssetId::new();
    let bundle = BundleWriter::build_bytes(
        "registry_textures",
        CompressionKind::None,
        vec![BundleAsset {
            id: texture_id,
            asset_type: AssetTypeId::of::<Texture>(),
            path: AssetPath::parse("textures/registry.texture"),
            bytes: texture_bytes(1, 1, 17),
            dependencies: vec![shader_id],
        }],
    )
    .unwrap();
    let entry_hash = BundleReader::from_bytes(&bundle)
        .unwrap()
        .manifest()
        .entry(texture_id)
        .unwrap()
        .content_hash;

    let mut server = AssetServer::new(AssetServerConfig::default());
    let mounted = server.mount_bundle_bytes(&bundle).unwrap();
    server.save_mounted_bundle_registry(&path).unwrap();

    let snapshot = MountedBundleRegistry::load_from_file(&path).unwrap();
    assert_eq!(snapshot.bundles().len(), 1);
    let snapshot_bundle = &snapshot.bundles()[0];
    assert_eq!(snapshot_bundle.id, mounted.id);
    assert_eq!(snapshot_bundle.name, "registry_textures");
    let snapshot_entry = snapshot_bundle.manifest.entry(texture_id).unwrap();
    assert_eq!(
        snapshot_entry.path,
        Some(AssetPath::parse("textures/registry.texture"))
    );
    assert_eq!(snapshot_entry.content_hash, entry_hash);
    assert_eq!(snapshot_entry.dependencies, vec![shader_id]);

    let mut restored_server = AssetServer::new(AssetServerConfig::default());
    restored_server.set_io(BundleAssetIo::from_bytes(&bundle).unwrap());
    restored_server.register_builtin_loaders();
    let restored = restored_server.load_mounted_bundle_registry(&path).unwrap();
    assert_eq!(restored, snapshot.bundles());
    assert!(restored_server.mounted_bundle(mounted.id).is_some());

    let missing_path = temp_file("missing_mounted_bundle_registry", "txt");
    let missing_error = restored_server
        .load_mounted_bundle_registry(&missing_path)
        .unwrap_err();
    assert!(matches!(
        missing_error,
        AssetError::Io { message }
            if message.contains("failed to read") && message.contains(&missing_path.display().to_string())
    ));
    assert!(restored_server.mounted_bundle(mounted.id).is_some());

    let remounted = restored_server.mounted_bundle(mounted.id).unwrap().clone();
    let group = restored_server.preload_bundle(&remounted);
    let metadata = restored_server.metadata(texture_id).unwrap();
    assert_eq!(
        metadata.path,
        Some(AssetPath::parse("textures/registry.texture"))
    );
    assert_eq!(metadata.cooked_hash, Some(entry_hash));
    assert_eq!(metadata.dependencies, vec![shader_id]);

    restored_server.update_loading();
    let uploads = restored_server.drain_gpu_uploads().collect::<Vec<_>>();
    restored_server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(21))),
    );
    assert_eq!(restored_server.group_state(&group), AssetLoadState::Ready);
    let ready_metadata = restored_server.metadata(texture_id).unwrap();
    assert_eq!(ready_metadata.source_hash, Some(entry_hash));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn asset_server_save_and_load_package_registry_round_trip_preserves_mounted_bundles() {
    let path = temp_file("asset_server_package_registry", "txt");
    let _ = std::fs::remove_file(&path);
    let (base, base_bundle, base_ids) = texture_package(
        "server_base",
        AssetIoLayerKind::BaseBundle,
        0,
        BundleId(61),
        "packages/server_base.nga_bundle",
        vec![("textures/server_base.texture", texture_bytes(1, 1, 11))],
    );
    let (patch, patch_bundle, patch_ids) = texture_package(
        "server_patch",
        AssetIoLayerKind::Patch,
        1,
        BundleId(62),
        "packages/server_patch.nga_bundle",
        vec![("textures/server_patch.texture", texture_bytes(1, 1, 22))],
    );
    let base_entry_hash = BundleReader::from_bytes(&base_bundle)
        .unwrap()
        .manifest()
        .entry(base_ids[0])
        .unwrap()
        .content_hash;
    let registry = AssetPackageRegistry::new(vec![base.clone(), patch.clone()]).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    let mounted = server
        .restore_asset_package_registry(registry.clone())
        .unwrap();

    server.save_asset_package_registry(&path).unwrap();
    let saved_registry = AssetPackageRegistry::load_from_file(&path).unwrap();
    assert_eq!(saved_registry, registry);

    let mut restored_server = AssetServer::new(AssetServerConfig::default());
    let restored = restored_server.load_asset_package_registry(&path).unwrap();
    assert_eq!(restored, mounted);
    assert_eq!(restored_server.mounted_bundles().count(), 2);
    assert_eq!(
        restored_server.mounted_bundle(base.bundle_id).unwrap().name,
        "server_base"
    );
    assert_eq!(
        restored_server
            .mounted_bundle(patch.bundle_id)
            .unwrap()
            .name,
        "server_patch"
    );
    assert_eq!(
        restored_server
            .mounted_bundle(base.bundle_id)
            .unwrap()
            .manifest
            .entry(base_ids[0])
            .unwrap()
            .content_hash,
        base_entry_hash
    );
    assert_eq!(
        restored_server
            .mounted_bundle(patch.bundle_id)
            .unwrap()
            .manifest
            .entry(patch_ids[0])
            .unwrap()
            .content_hash,
        BundleReader::from_bytes(&patch_bundle)
            .unwrap()
            .manifest()
            .entry(patch_ids[0])
            .unwrap()
            .content_hash
    );

    let missing_path = temp_file("asset_server_missing_package_registry", "txt");
    let missing_error = restored_server
        .load_asset_package_registry(&missing_path)
        .unwrap_err();
    assert!(matches!(
        missing_error,
        AssetError::Io { message }
            if message.contains("failed to read") && message.contains(&missing_path.display().to_string())
    ));
    assert_eq!(restored_server.mounted_bundles().count(), 2);
}

#[test]
fn asset_server_reports_invalid_bundle_mount_errors() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    let error = server.mount_bundle_bytes(b"not a bundle").unwrap_err();

    assert!(matches!(error, AssetError::Bundle { .. }));
    assert!(matches!(
        server.unmount_bundle(BundleId(999)),
        Err(AssetError::Bundle { .. })
    ));
}
