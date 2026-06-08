use crate::{
    asset::{Asset, AssetDependencies, AssetDependencyReference, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    handle::{HandleStrength, UntypedHandle},
    id::AssetTypeId,
    loader::{AssetLoader, LoadContext, LoadedAsset, LoaderSettings},
    path::AssetPath,
};

use super::scene::{
    dependency_type_for_path, parse_serialized_component, parse_serialized_entity,
    serialized_component_asset_dependencies, SerializedEntity,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Prefab {
    pub root: SerializedEntity,
    pub children: Vec<SerializedEntity>,
    pub dependencies: Vec<UntypedHandle>,
}

impl Asset for Prefab {
    const TYPE_NAME: &'static str = "Prefab";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0009);
}

impl AssetMemoryUsage for Prefab {
    fn cpu_bytes(&self) -> u64 {
        self.children.len() as u64 * 64
    }
}

impl AssetDependencies for Prefab {
    fn visit_dependencies(&self, visitor: &mut dyn FnMut(AssetDependencyReference)) {
        for dependency in &self.dependencies {
            visitor(AssetDependencyReference::from_handle(dependency.clone()));
        }
    }
}

pub struct PrefabLoader;

impl PrefabLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PrefabLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for PrefabLoader {
    fn name(&self) -> &'static str {
        "PrefabLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["prefab"]
    }

    fn asset_type(&self) -> AssetTypeId {
        Prefab::TYPE_ID
    }

    fn load(
        &self,
        ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        parse_prefab(ctx, bytes).map(LoadedAsset::new)
    }
}

#[derive(Clone, Copy)]
enum PrefabEntityTarget {
    Root,
    Child(usize),
}

fn parse_prefab(ctx: &mut LoadContext<'_>, bytes: &[u8]) -> Result<Prefab, AssetError> {
    let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
        message: format!("prefab source must be UTF-8: {error}"),
    })?;
    let mut lines = source.lines();
    let header = lines.next().unwrap_or("").trim();
    if header != "NGA_PREFAB_V1" {
        return Err(AssetError::Decode {
            message: "prefab source must start with NGA_PREFAB_V1".to_owned(),
        });
    }

    let mut root = None;
    let mut children = Vec::new();
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
                message: format!("invalid prefab line {line_number}"),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "dependency" => {
                let path = AssetPath::parse(value);
                let asset_type = dependency_type_for_path(&path, line_number, "prefab")?;
                add_prefab_dependency(
                    ctx,
                    &mut dependencies,
                    &mut dependency_keys,
                    path,
                    asset_type,
                );
            }
            "root" => {
                if root.is_some() {
                    return Err(AssetError::Decode {
                        message: format!("duplicate prefab root on line {line_number}"),
                    });
                }
                let root_entity = parse_serialized_entity(value, line_number, "prefab")?;
                if root_entity.parent.is_some() {
                    return Err(AssetError::Decode {
                        message: format!("prefab root cannot have parent on line {line_number}"),
                    });
                }
                root = Some(root_entity);
                current_entity = Some(PrefabEntityTarget::Root);
            }
            "child" => {
                if root.is_none() {
                    return Err(AssetError::Decode {
                        message: format!("prefab child on line {line_number} has no root"),
                    });
                }
                let child = parse_serialized_entity(value, line_number, "prefab")?;
                children.push(child);
                current_entity = Some(PrefabEntityTarget::Child(children.len() - 1));
            }
            "component" => {
                let component = parse_serialized_component(value, line_number, "prefab")?;
                for (path, asset_type) in
                    serialized_component_asset_dependencies(&component, line_number, "prefab")?
                {
                    add_prefab_dependency(
                        ctx,
                        &mut dependencies,
                        &mut dependency_keys,
                        path,
                        asset_type,
                    );
                }
                match current_entity {
                    Some(PrefabEntityTarget::Root) => {
                        if let Some(root) = root.as_mut() {
                            root.components.push(component);
                        }
                    }
                    Some(PrefabEntityTarget::Child(index)) => {
                        children[index].components.push(component);
                    }
                    None => {
                        return Err(AssetError::Decode {
                            message: format!(
                                "prefab component on line {line_number} has no entity"
                            ),
                        });
                    }
                }
            }
            other => {
                return Err(AssetError::Decode {
                    message: format!("unknown prefab key `{other}` on line {line_number}"),
                })
            }
        }
    }

    let root = root.ok_or_else(|| AssetError::Decode {
        message: "prefab source missing root".to_owned(),
    })?;
    Ok(Prefab {
        root,
        children,
        dependencies,
    })
}

fn add_prefab_dependency(
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
