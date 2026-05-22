use crate::{
    error::{AssetError, AssetResult},
    id::AssetTypeId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssetFeature {
    Filesystem,
    Bundle,
    HotReload,
    Streaming,
    Editor,
    Importers,
    Cookers,
    TextureImporter,
    ModelImporter,
    MaterialImporter,
    AudioImporter,
    ShaderImporter,
    TextureCooker,
    ModelCooker,
    MaterialCooker,
    AudioCooker,
    ShaderCooker,
    AsyncLoading,
    Parallel,
    Serde,
    Zstd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetFeatureStatus {
    pub feature: AssetFeature,
    pub name: &'static str,
    pub enabled: bool,
}

impl AssetFeature {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Filesystem => "filesystem",
            Self::Bundle => "bundle",
            Self::HotReload => "hot_reload",
            Self::Streaming => "streaming",
            Self::Editor => "editor",
            Self::Importers => "importers",
            Self::Cookers => "cookers",
            Self::TextureImporter => "texture_importer",
            Self::ModelImporter => "model_importer",
            Self::MaterialImporter => "material_importer",
            Self::AudioImporter => "audio_importer",
            Self::ShaderImporter => "shader_importer",
            Self::TextureCooker => "texture_cooker",
            Self::ModelCooker => "model_cooker",
            Self::MaterialCooker => "material_cooker",
            Self::AudioCooker => "audio_cooker",
            Self::ShaderCooker => "shader_cooker",
            Self::AsyncLoading => "async_loading",
            Self::Parallel => "parallel",
            Self::Serde => "serde",
            Self::Zstd => "zstd",
        }
    }

    pub const fn unsupported_message(self) -> &'static str {
        match self {
            Self::Filesystem => "asset filesystem feature is disabled",
            Self::Bundle => "asset bundle feature is disabled",
            Self::HotReload => "asset hot_reload feature is disabled",
            Self::Streaming => "asset streaming feature is disabled",
            Self::Editor => "asset editor feature is disabled",
            Self::Importers => "asset importers feature is disabled",
            Self::Cookers => "asset cookers feature is disabled",
            Self::TextureImporter => "asset texture_importer feature is disabled",
            Self::ModelImporter => "asset model_importer feature is disabled",
            Self::MaterialImporter => "asset material_importer feature is disabled",
            Self::AudioImporter => "asset audio_importer feature is disabled",
            Self::ShaderImporter => "asset shader_importer feature is disabled",
            Self::TextureCooker => "asset texture_cooker feature is disabled",
            Self::ModelCooker => "asset model_cooker feature is disabled",
            Self::MaterialCooker => "asset material_cooker feature is disabled",
            Self::AudioCooker => "asset audio_cooker feature is disabled",
            Self::ShaderCooker => "asset shader_cooker feature is disabled",
            Self::AsyncLoading => "asset async_loading feature is disabled",
            Self::Parallel => "asset parallel feature is disabled",
            Self::Serde => "asset serde feature is disabled",
            Self::Zstd => "asset zstd feature is disabled",
        }
    }
}

pub fn asset_feature_status(feature: AssetFeature) -> AssetFeatureStatus {
    AssetFeatureStatus {
        feature,
        name: feature.name(),
        enabled: asset_feature_enabled(feature),
    }
}

pub fn asset_feature_enabled(feature: AssetFeature) -> bool {
    match feature {
        AssetFeature::Filesystem => cfg!(feature = "filesystem"),
        AssetFeature::Bundle => cfg!(feature = "bundle"),
        AssetFeature::HotReload => cfg!(feature = "hot_reload"),
        AssetFeature::Streaming => cfg!(feature = "streaming"),
        AssetFeature::Editor => cfg!(feature = "editor"),
        AssetFeature::Importers => cfg!(feature = "importers"),
        AssetFeature::Cookers => cfg!(feature = "cookers"),
        AssetFeature::TextureImporter => cfg!(feature = "texture_importer"),
        AssetFeature::ModelImporter => cfg!(feature = "model_importer"),
        AssetFeature::MaterialImporter => cfg!(feature = "material_importer"),
        AssetFeature::AudioImporter => cfg!(feature = "audio_importer"),
        AssetFeature::ShaderImporter => cfg!(feature = "shader_importer"),
        AssetFeature::TextureCooker => cfg!(feature = "texture_cooker"),
        AssetFeature::ModelCooker => cfg!(feature = "model_cooker"),
        AssetFeature::MaterialCooker => cfg!(feature = "material_cooker"),
        AssetFeature::AudioCooker => cfg!(feature = "audio_cooker"),
        AssetFeature::ShaderCooker => cfg!(feature = "shader_cooker"),
        AssetFeature::AsyncLoading => cfg!(feature = "async_loading"),
        AssetFeature::Parallel => cfg!(feature = "parallel"),
        AssetFeature::Serde => cfg!(feature = "serde"),
        AssetFeature::Zstd => cfg!(feature = "zstd"),
    }
}

pub fn require_asset_feature(feature: AssetFeature) -> AssetResult<()> {
    if asset_feature_enabled(feature) {
        Ok(())
    } else {
        Err(AssetError::Unsupported(feature.unsupported_message()))
    }
}

pub fn importer_feature_for_asset_type(asset_type: AssetTypeId) -> Option<AssetFeature> {
    use crate::asset::Asset;
    use crate::assets::{AudioClip, Material, Mesh, Shader, Texture};

    if asset_type == Texture::TYPE_ID {
        Some(AssetFeature::TextureImporter)
    } else if asset_type == Mesh::TYPE_ID {
        Some(AssetFeature::ModelImporter)
    } else if asset_type == Material::TYPE_ID {
        Some(AssetFeature::MaterialImporter)
    } else if asset_type == AudioClip::TYPE_ID {
        Some(AssetFeature::AudioImporter)
    } else if asset_type == Shader::TYPE_ID {
        Some(AssetFeature::ShaderImporter)
    } else {
        None
    }
}
