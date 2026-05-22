#![cfg(feature = "serde")]

use engine_asset::prelude::*;

fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    let json = serde_json::to_string(value).unwrap();
    serde_json::from_str(&json).unwrap()
}

#[test]
fn serde_feature_round_trips_asset_reference_identity_path_and_metadata() {
    let id = AssetId::from_u128(0x4e47_4153_5345_5400_0000_0000_0000_0101);
    let dependency = AssetId::from_u128(0x4e47_4153_5345_5400_0000_0000_0000_0102);
    let path = AssetPath::with_label("textures\\hero.texture", "albedo");
    let reference = AssetRef::<Texture>::with_fallback(id, path.clone());

    let decoded_reference = round_trip(&reference);
    assert_eq!(decoded_reference.id(), id);
    assert_eq!(decoded_reference.fallback_path.as_ref(), Some(&path));

    assert_eq!(round_trip(&id), id);
    assert_eq!(round_trip(&Texture::TYPE_ID), Texture::TYPE_ID);
    assert_eq!(
        round_trip(&AssetTypeName::new("Texture")),
        AssetTypeName::new("Texture")
    );
    assert_eq!(round_trip(&ContentHash(123)), ContentHash(123));
    assert_eq!(round_trip(&VersionHash(456)), VersionHash(456));
    assert_eq!(round_trip(&path), path);

    let metadata = AssetMetadata {
        id,
        path: Some(path.clone()),
        asset_type: Texture::TYPE_ID,
        source_path: Some(AssetPath::parse("source/hero.png")),
        cooked_path: Some(AssetPath::parse("cooked/hero.texture")),
        importer: Some("TextureImporter".to_owned()),
        importer_version: 7,
        source_hash: Some(ContentHash(111)),
        settings_hash: Some(ContentHash(222)),
        cooked_hash: Some(ContentHash(333)),
        version_hash: Some(VersionHash(444)),
        labels: vec!["hero".to_owned(), "runtime".to_owned()],
        dependencies: vec![dependency],
        importer_settings: vec![
            ("format".to_owned(), "rgba8".to_owned()),
            ("mips".to_owned(), "true".to_owned()),
        ],
    };

    assert_eq!(round_trip(&metadata), metadata);
}
