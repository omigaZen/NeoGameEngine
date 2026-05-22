use engine_asset::prelude::*;

fn texture(width: u32, height: u32, value: u8) -> Texture {
    Texture {
        width,
        height,
        format: TextureFormat::Rgba8UnormSrgb,
        mip_count: 1,
        data: vec![value; width as usize * height as usize * 4],
        gpu: None,
    }
}

#[test]
fn typed_storage_insert_get_iter_mutate_and_remove() {
    let mut storage = Assets::<Texture>::new();
    let ready_id = AssetId::new();
    let cpu_only_id = AssetId::new();
    let ready_handle = Handle::<Texture>::strong(ready_id);

    assert!(storage.is_empty());
    assert_eq!(storage.insert(ready_id, texture(1, 1, 10)), None);
    assert_eq!(
        storage.insert_with_state(cpu_only_id, texture(2, 1, 20), AssetLoadState::LoadedCpu),
        None
    );

    assert_eq!(storage.len(), 2);
    assert!(storage.contains(ready_id));
    assert_eq!(storage.state(ready_id), AssetLoadState::Ready);
    assert_eq!(storage.get(&ready_handle).unwrap().width, 1);
    assert!(storage.get_by_id(cpu_only_id).is_none());
    assert_eq!(storage.get_cpu_by_id(cpu_only_id).unwrap().width, 2);

    storage.get_mut(&ready_handle).unwrap().mip_count = 3;
    for (_, texture) in storage.iter_mut() {
        texture.height += 1;
    }
    let mut iterated = storage
        .iter()
        .map(|(id, texture)| (id, texture.height))
        .collect::<Vec<_>>();
    iterated.sort_by_key(|(id, _)| *id);
    assert_eq!(iterated.len(), 2);
    assert_eq!(storage.get_by_id(ready_id).unwrap().mip_count, 3);
    assert_eq!(storage.get_by_id(ready_id).unwrap().height, 2);
    assert_eq!(storage.get_cpu_by_id(cpu_only_id).unwrap().height, 2);

    let removed = storage.remove(ready_id).unwrap();
    assert_eq!(removed.width, 1);
    assert_eq!(storage.len(), 1);
    assert!(!storage.contains(ready_id));
    assert_eq!(storage.state(ready_id), AssetLoadState::Unloaded);
}

#[test]
fn typed_storage_entries_track_metadata_counts_errors_and_usage() {
    let mut storage = Assets::<Texture>::new();
    let id = AssetId::new();
    let path = AssetPath::parse("textures/entry.texture");
    let metadata = AssetMetadata::runtime(id, path.clone(), AssetTypeId::of::<Texture>());

    let entry = storage.ensure_entry(id);
    entry.metadata = Some(metadata.clone());
    entry.strong_count = 2;
    entry.weak_count = 1;
    entry.dependency_ref_count = 3;
    entry.resident = true;
    entry.error = Some(AssetError::Decode {
        message: "bad texture".to_owned(),
    });
    entry.state = AssetLoadState::Failed;
    storage.mark_used(id, 42);

    let entry = storage.entry(id).unwrap();
    assert_eq!(entry.metadata.as_ref().unwrap().path.as_ref(), Some(&path));
    assert_eq!(entry.strong_count, 2);
    assert_eq!(entry.weak_count, 1);
    assert_eq!(entry.dependency_ref_count, 3);
    assert!(entry.resident);
    assert_eq!(entry.last_used_frame, 42);
    assert!(matches!(
        storage.error(id),
        Some(AssetError::Decode { message }) if message == "bad texture"
    ));
    assert_eq!(storage.state(id), AssetLoadState::Failed);
}
