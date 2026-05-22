use crate::{
    asset::{Asset, AssetDependencies, AssetDependencyReference, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    handle::{HandleStrength, UntypedHandle},
    id::AssetTypeId,
    loader::{AssetLoader, LoadContext, LoadedAsset, LoaderSettings},
    path::AssetPath,
};

use super::{
    audio::AudioClip, material::Material, mesh::Mesh, prefab::Prefab, shader::Shader,
    texture::Texture,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SceneAsset {
    pub name: String,
    pub entities: Vec<SerializedEntity>,
    pub dependencies: Vec<UntypedHandle>,
}

impl Asset for SceneAsset {
    const TYPE_NAME: &'static str = "SceneAsset";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0008);
}

impl AssetMemoryUsage for SceneAsset {
    fn cpu_bytes(&self) -> u64 {
        self.entities
            .iter()
            .map(|entity| {
                entity.name.as_ref().map(|name| name.len()).unwrap_or(0) as u64
                    + entity
                        .components
                        .iter()
                        .map(|component| {
                            component.type_name.len() as u64 + component.data.len() as u64
                        })
                        .sum::<u64>()
            })
            .sum()
    }
}

impl AssetDependencies for SceneAsset {
    fn visit_dependencies(&self, visitor: &mut dyn FnMut(AssetDependencyReference)) {
        for dependency in &self.dependencies {
            visitor(AssetDependencyReference::from_handle(dependency.clone()));
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerializedEntity {
    pub name: Option<String>,
    pub parent: Option<u64>,
    pub components: Vec<SerializedComponent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerializedComponent {
    pub type_name: String,
    pub data: Vec<u8>,
}

pub struct SceneLoader;

impl SceneLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SceneLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for SceneLoader {
    fn name(&self) -> &'static str {
        "SceneLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["scene"]
    }

    fn asset_type(&self) -> AssetTypeId {
        SceneAsset::TYPE_ID
    }

    fn load(
        &self,
        ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        parse_scene_asset(ctx, bytes).map(LoadedAsset::new)
    }
}

fn parse_scene_asset(ctx: &mut LoadContext<'_>, bytes: &[u8]) -> Result<SceneAsset, AssetError> {
    let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
        message: format!("scene source must be UTF-8: {error}"),
    })?;
    let mut lines = source.lines();
    let header = lines.next().unwrap_or("").trim();
    if header != "NGA_SCENE_V1" {
        return Err(AssetError::Decode {
            message: "scene source must start with NGA_SCENE_V1".to_owned(),
        });
    }

    let mut name = None;
    let mut entities = Vec::new();
    let mut dependencies = Vec::new();
    let mut current_entity = None;

    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Decode {
                message: format!("invalid scene line {line_number}"),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "name" => {
                if value.is_empty() {
                    return Err(AssetError::Decode {
                        message: format!("scene name is empty on line {line_number}"),
                    });
                }
                name = Some(value.to_owned());
            }
            "dependency" => {
                let path = AssetPath::parse(value);
                let asset_type = dependency_type_for_path(&path, line_number, "scene")?;
                let id = ctx.add_dependency(path, asset_type);
                dependencies.push(UntypedHandle::new(id, asset_type, HandleStrength::Weak));
            }
            "entity" => {
                let entity = parse_serialized_entity(value, line_number, "scene")?;
                entities.push(entity);
                current_entity = Some(entities.len() - 1);
            }
            "component" => {
                let Some(entity_index) = current_entity else {
                    return Err(AssetError::Decode {
                        message: format!("scene component on line {line_number} has no entity"),
                    });
                };
                entities[entity_index]
                    .components
                    .push(parse_serialized_component(value, line_number, "scene")?);
            }
            other => {
                return Err(AssetError::Decode {
                    message: format!("unknown scene key `{other}` on line {line_number}"),
                })
            }
        }
    }

    let name = name.ok_or_else(|| AssetError::Decode {
        message: "scene source missing name".to_owned(),
    })?;
    Ok(SceneAsset {
        name,
        entities,
        dependencies,
    })
}

pub(crate) fn parse_serialized_entity(
    value: &str,
    line_number: usize,
    asset_kind: &str,
) -> Result<SerializedEntity, AssetError> {
    let mut parts = value.split(';').map(str::trim);
    let name = parts.next().unwrap_or("");
    if name.is_empty() {
        return Err(AssetError::Decode {
            message: format!("{asset_kind} entity name is empty on line {line_number}"),
        });
    }
    let mut parent = None;
    for part in parts {
        let Some((key, value)) = part.split_once('=') else {
            return Err(AssetError::Decode {
                message: format!("invalid {asset_kind} entity field on line {line_number}"),
            });
        };
        match (key.trim(), value.trim()) {
            ("parent", value) => {
                parent = Some(value.parse().map_err(|error| AssetError::Decode {
                    message: format!(
                        "invalid {asset_kind} entity parent on line {line_number}: {error}"
                    ),
                })?);
            }
            (other, _) => {
                return Err(AssetError::Decode {
                    message: format!(
                        "unknown {asset_kind} entity field `{other}` on line {line_number}"
                    ),
                })
            }
        }
    }
    Ok(SerializedEntity {
        name: Some(name.to_owned()),
        parent,
        components: Vec::new(),
    })
}

pub(crate) fn parse_serialized_component(
    value: &str,
    line_number: usize,
    asset_kind: &str,
) -> Result<SerializedComponent, AssetError> {
    let Some((type_name, data)) = value.split_once('|') else {
        return Err(AssetError::Decode {
            message: format!("invalid {asset_kind} component on line {line_number}"),
        });
    };
    let type_name = type_name.trim();
    if type_name.is_empty() {
        return Err(AssetError::Decode {
            message: format!("{asset_kind} component type is empty on line {line_number}"),
        });
    }
    Ok(SerializedComponent {
        type_name: type_name.to_owned(),
        data: data.as_bytes().to_vec(),
    })
}

pub(crate) fn dependency_type_for_path(
    path: &AssetPath,
    line_number: usize,
    asset_kind: &str,
) -> Result<AssetTypeId, AssetError> {
    match path.extension().map(str::to_ascii_lowercase).as_deref() {
        Some("texture" | "tex" | "rgba") => Ok(Texture::TYPE_ID),
        Some("mesh") => Ok(Mesh::TYPE_ID),
        Some("wgsl" | "glsl" | "shader") => Ok(Shader::TYPE_ID),
        Some("material" | "mat") => Ok(Material::TYPE_ID),
        Some("audio" | "wav" | "ogg") => Ok(AudioClip::TYPE_ID),
        Some("scene") => Ok(SceneAsset::TYPE_ID),
        Some("prefab") => Ok(Prefab::TYPE_ID),
        Some(extension) => Err(AssetError::Decode {
            message: format!(
                "unsupported {asset_kind} dependency extension `{extension}` on line {line_number}"
            ),
        }),
        None => Err(AssetError::Decode {
            message: format!(
                "{asset_kind} dependency path missing extension on line {line_number}"
            ),
        }),
    }
}
