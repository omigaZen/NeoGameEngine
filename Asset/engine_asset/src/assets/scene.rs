use crate::{
    asset::{Asset, AssetDependencies, AssetDependencyReference, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    handle::{HandleStrength, UntypedHandle},
    id::AssetTypeId,
    loader::{AssetLoader, LoadContext, LoadedAsset, LoaderSettings},
    path::AssetPath,
};

use super::{
    audio::AudioClip, material::Material, mesh::Mesh, physics_mesh::PhysicsMesh, prefab::Prefab,
    shader::Shader, skeleton::Skeleton, texture::Texture,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerializedComponentAssetField {
    pub component_type: &'static str,
    pub field: &'static str,
    pub asset_type: AssetTypeId,
    pub asset_type_name: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerializedComponentAssetReference {
    pub component_type: String,
    pub field: String,
    pub path: AssetPath,
    pub asset_type: AssetTypeId,
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
    let mut dependency_keys = Vec::new();
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
        match scene_prefab_document_key(key).as_str() {
            "name" | "scenename" => {
                if value.is_empty() {
                    return Err(AssetError::Decode {
                        message: format!("scene name is empty on line {line_number}"),
                    });
                }
                name = Some(value.to_owned());
            }
            key if is_scene_prefab_dependency_key(key) => {
                let path = AssetPath::parse(value);
                let asset_type = dependency_type_for_path(&path, line_number, "scene")?;
                add_serialized_dependency(
                    ctx,
                    &mut dependencies,
                    &mut dependency_keys,
                    path,
                    asset_type,
                );
            }
            "entity" | "node" | "gameobject" => {
                let entity = parse_serialized_entity(value, line_number, "scene")?;
                entities.push(entity);
                current_entity = Some(entities.len() - 1);
            }
            "component" | "cmp" => {
                let Some(entity_index) = current_entity else {
                    return Err(AssetError::Decode {
                        message: format!("scene component on line {line_number} has no entity"),
                    });
                };
                let component = parse_serialized_component(value, line_number, "scene")?;
                for (path, asset_type) in
                    serialized_component_asset_dependencies(&component, line_number, "scene")?
                {
                    add_serialized_dependency(
                        ctx,
                        &mut dependencies,
                        &mut dependency_keys,
                        path,
                        asset_type,
                    );
                }
                entities[entity_index].components.push(component);
            }
            _ => {
                return Err(AssetError::Decode {
                    message: format!("unknown scene key `{key}` on line {line_number}"),
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
        let key = key.trim();
        let value = value.trim();
        match scene_prefab_document_key(key).as_str() {
            "parent" | "parentid" | "parentindex" => {
                parent = Some(value.parse().map_err(|error| AssetError::Decode {
                    message: format!(
                        "invalid {asset_kind} entity parent on line {line_number}: {error}"
                    ),
                })?);
            }
            _ => {
                return Err(AssetError::Decode {
                    message: format!(
                        "unknown {asset_kind} entity field `{key}` on line {line_number}"
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

pub(crate) fn serialized_component_asset_dependencies(
    component: &SerializedComponent,
    line_number: usize,
    asset_kind: &str,
) -> Result<Vec<(AssetPath, AssetTypeId)>, AssetError> {
    let references =
        serialized_component_asset_references_with_context(component, line_number, asset_kind)?;
    let mut dependencies = Vec::new();
    for reference in references {
        if !dependencies.iter().any(|(existing_path, existing_type)| {
            existing_path == &reference.path && *existing_type == reference.asset_type
        }) {
            dependencies.push((reference.path, reference.asset_type));
        }
    }
    Ok(dependencies)
}

pub fn serialized_component_asset_references(
    component: &SerializedComponent,
) -> Result<Vec<SerializedComponentAssetReference>, AssetError> {
    serialized_component_asset_references_with_context(component, 1, "serialized component")
}

fn serialized_component_asset_references_with_context(
    component: &SerializedComponent,
    line_number: usize,
    asset_kind: &str,
) -> Result<Vec<SerializedComponentAssetReference>, AssetError> {
    let data = std::str::from_utf8(&component.data).map_err(|error| AssetError::Decode {
        message: format!(
            "{asset_kind} component `{}` data must be UTF-8 on line {line_number}: {error}",
            component.type_name
        ),
    })?;
    let mut references = Vec::new();
    for field in data
        .split(';')
        .map(str::trim)
        .filter(|field| !field.is_empty())
    {
        let Some((key, value)) = field.split_once('=') else {
            if serialized_component_type_has_asset_fields(&component.type_name) {
                return Err(AssetError::Decode {
                    message: format!(
                        "{asset_kind} {} component field `{field}` on line {line_number} must be key=value",
                        component.type_name
                    ),
                });
            }
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        let Some(asset_type) = serialized_component_asset_field_type(&component.type_name, key)
        else {
            continue;
        };
        if value.is_empty() {
            return Err(AssetError::Decode {
                message: format!(
                    "{asset_kind} {} component asset field `{key}` is empty on line {line_number}",
                    component.type_name
                ),
            });
        }
        let path = AssetPath::parse(value);
        let actual_type = dependency_type_for_path(
            &path,
            line_number,
            &format!("{asset_kind} {}.{key}", component.type_name),
        )?;
        if actual_type != asset_type {
            return Err(AssetError::Decode {
                message: format!(
                    "{asset_kind} {} component field `{key}` on line {line_number} expects {} but `{}` resolves to {}",
                    component.type_name,
                    asset_type_name(asset_type),
                    path.display_string(),
                    asset_type_name(actual_type)
                ),
            });
        }
        references.push(SerializedComponentAssetReference {
            component_type: component.type_name.clone(),
            field: key.to_owned(),
            path,
            asset_type,
        });
    }
    Ok(references)
}

pub fn serialized_component_type_has_asset_fields(type_name: &str) -> bool {
    matches!(
        scene_prefab_document_key(type_name).as_str(),
        "meshrenderer"
            | "skinnedmeshrenderer"
            | "audiosource"
            | "physicscollider"
            | "sceneinstance"
            | "prefabinstance"
    )
}

pub fn serialized_component_asset_field_type(type_name: &str, field: &str) -> Option<AssetTypeId> {
    match (
        scene_prefab_document_key(type_name).as_str(),
        scene_prefab_document_key(field).as_str(),
    ) {
        ("meshrenderer", "mesh") => Some(Mesh::TYPE_ID),
        ("meshrenderer", "material") => Some(Material::TYPE_ID),
        ("skinnedmeshrenderer", "mesh") => Some(Mesh::TYPE_ID),
        ("skinnedmeshrenderer", "skeleton") => Some(Skeleton::TYPE_ID),
        ("skinnedmeshrenderer", "material") => Some(Material::TYPE_ID),
        ("audiosource", "clip" | "audio" | "audioclip") => Some(AudioClip::TYPE_ID),
        ("physicscollider", "mesh" | "physicsmesh" | "collisionmesh") => Some(PhysicsMesh::TYPE_ID),
        ("sceneinstance", "scene") => Some(SceneAsset::TYPE_ID),
        ("prefabinstance", "prefab") => Some(Prefab::TYPE_ID),
        _ => None,
    }
}

pub fn serialized_component_asset_fields(type_name: &str) -> Vec<SerializedComponentAssetField> {
    let fields: &[(&str, &str, AssetTypeId)] = match scene_prefab_document_key(type_name).as_str() {
        "meshrenderer" => &[
            ("MeshRenderer", "mesh", Mesh::TYPE_ID),
            ("MeshRenderer", "material", Material::TYPE_ID),
        ],
        "skinnedmeshrenderer" => &[
            ("SkinnedMeshRenderer", "mesh", Mesh::TYPE_ID),
            ("SkinnedMeshRenderer", "skeleton", Skeleton::TYPE_ID),
            ("SkinnedMeshRenderer", "material", Material::TYPE_ID),
        ],
        "audiosource" => &[("AudioSource", "clip", AudioClip::TYPE_ID)],
        "physicscollider" => &[
            ("PhysicsCollider", "mesh", PhysicsMesh::TYPE_ID),
            ("PhysicsCollider", "physics_mesh", PhysicsMesh::TYPE_ID),
        ],
        "sceneinstance" => &[("SceneInstance", "scene", SceneAsset::TYPE_ID)],
        "prefabinstance" => &[("PrefabInstance", "prefab", Prefab::TYPE_ID)],
        _ => &[],
    };
    fields
        .iter()
        .map(
            |(component_type, field, asset_type)| SerializedComponentAssetField {
                component_type,
                field,
                asset_type: *asset_type,
                asset_type_name: asset_type_name(*asset_type),
            },
        )
        .collect()
}

pub(crate) fn scene_prefab_document_key(key: &str) -> String {
    key.chars()
        .filter(|character| {
            !character.is_ascii_whitespace() && *character != '_' && *character != '-'
        })
        .flat_map(char::to_lowercase)
        .collect()
}

pub(crate) fn is_scene_prefab_dependency_key(key: &str) -> bool {
    let key = scene_prefab_document_key(key);
    matches!(
        key.as_str(),
        "dependency"
            | "dependencies"
            | "depends"
            | "depend"
            | "reference"
            | "references"
            | "ref"
            | "refs"
    )
}

fn add_serialized_dependency(
    ctx: &mut LoadContext<'_>,
    dependencies: &mut Vec<UntypedHandle>,
    dependency_keys: &mut Vec<(AssetPath, AssetTypeId)>,
    path: AssetPath,
    asset_type: AssetTypeId,
) {
    if dependency_keys
        .iter()
        .any(|(existing_path, existing_type)| {
            existing_path == &path && *existing_type == asset_type
        })
    {
        return;
    }
    let id = ctx.add_dependency(path.clone(), asset_type);
    dependencies.push(UntypedHandle::new(id, asset_type, HandleStrength::Weak));
    dependency_keys.push((path, asset_type));
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
        Some("skeleton" | "skel") => Ok(Skeleton::TYPE_ID),
        Some("physics" | "physicsmesh" | "pmesh") => Ok(PhysicsMesh::TYPE_ID),
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

fn asset_type_name(asset_type: AssetTypeId) -> &'static str {
    match asset_type {
        t if t == Texture::TYPE_ID => Texture::TYPE_NAME,
        t if t == Mesh::TYPE_ID => Mesh::TYPE_NAME,
        t if t == Shader::TYPE_ID => Shader::TYPE_NAME,
        t if t == Material::TYPE_ID => Material::TYPE_NAME,
        t if t == AudioClip::TYPE_ID => AudioClip::TYPE_NAME,
        t if t == SceneAsset::TYPE_ID => SceneAsset::TYPE_NAME,
        t if t == Prefab::TYPE_ID => Prefab::TYPE_NAME,
        t if t == Skeleton::TYPE_ID => Skeleton::TYPE_NAME,
        t if t == PhysicsMesh::TYPE_ID => PhysicsMesh::TYPE_NAME,
        _ => "Asset",
    }
}
