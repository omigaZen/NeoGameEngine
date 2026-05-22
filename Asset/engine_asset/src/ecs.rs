use crate::assets::{AudioClip, Material, Mesh, PhysicsMesh, Prefab, SceneAsset, Skeleton};
use crate::handle::{Handle, UntypedHandle};
use crate::id::AssetId;
use crate::server::AssetServer;

#[derive(Clone, Debug)]
pub struct MeshRendererComponent {
    pub mesh: Handle<Mesh>,
    pub material: Handle<Material>,
}

impl MeshRendererComponent {
    pub fn asset_handles(&self) -> Vec<UntypedHandle> {
        vec![self.mesh.untyped(), self.material.untyped()]
    }

    pub fn is_ready(&self, assets: &AssetServer) -> bool {
        assets.is_ready(&self.mesh) && assets.is_ready_with_dependencies(&self.material)
    }
}

#[derive(Clone, Debug)]
pub struct SkinnedMeshRendererComponent {
    pub mesh: Handle<Mesh>,
    pub skeleton: Handle<Skeleton>,
    pub material: Handle<Material>,
}

impl SkinnedMeshRendererComponent {
    pub fn asset_handles(&self) -> Vec<UntypedHandle> {
        vec![
            self.mesh.untyped(),
            self.skeleton.untyped(),
            self.material.untyped(),
        ]
    }

    pub fn is_ready(&self, assets: &AssetServer) -> bool {
        assets.is_ready(&self.mesh)
            && assets.is_ready(&self.skeleton)
            && assets.is_ready_with_dependencies(&self.material)
    }
}

#[derive(Clone, Debug)]
pub struct AudioSourceComponent {
    pub clip: Handle<AudioClip>,
    pub looping: bool,
    pub volume: f32,
}

impl AudioSourceComponent {
    pub fn asset_handles(&self) -> Vec<UntypedHandle> {
        vec![self.clip.untyped()]
    }

    pub fn is_ready(&self, assets: &AssetServer) -> bool {
        assets.is_ready(&self.clip)
    }
}

#[derive(Clone, Debug)]
pub struct PhysicsColliderComponent {
    pub mesh: Handle<PhysicsMesh>,
    pub dynamic: bool,
}

impl PhysicsColliderComponent {
    pub fn asset_handles(&self) -> Vec<UntypedHandle> {
        vec![self.mesh.untyped()]
    }

    pub fn is_ready(&self, assets: &AssetServer) -> bool {
        assets.is_ready(&self.mesh)
    }
}

#[derive(Clone, Debug)]
pub struct SceneInstanceComponent {
    pub scene: Handle<SceneAsset>,
    pub loaded: bool,
}

impl SceneInstanceComponent {
    pub fn asset_handles(&self) -> Vec<UntypedHandle> {
        vec![self.scene.untyped()]
    }

    pub fn is_scene_asset_ready(&self, assets: &AssetServer) -> bool {
        assets.is_ready_with_dependencies(&self.scene)
    }

    pub fn can_instantiate(&self, assets: &AssetServer) -> bool {
        !self.loaded && self.is_scene_asset_ready(assets)
    }

    pub fn instantiation_plan(&self, assets: &AssetServer) -> Option<SceneInstantiationPlan> {
        if !self.can_instantiate(assets) {
            return None;
        }
        let scene = assets.get(&self.scene)?;
        Some(SceneInstantiationPlan::from_scene(self.scene.id(), scene))
    }

    pub fn instantiation_commands(
        &self,
        assets: &AssetServer,
    ) -> Option<Vec<SceneInstantiationCommand>> {
        if !self.can_instantiate(assets) {
            return None;
        }
        let scene = assets.get(&self.scene)?;
        Some(SceneInstantiationPlan::from_scene(self.scene.id(), scene).commands(scene))
    }
}

pub trait InstantiationSink {
    fn spawn_entity(&mut self, entity_index: usize, name: Option<&str>, parent: Option<u64>);
    fn attach_component(&mut self, entity_index: usize, type_name: &str, data: &[u8]);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneInstantiationPlan {
    pub scene: AssetId,
    pub entity_count: usize,
    pub component_count: usize,
    pub dependency_count: usize,
}

impl SceneInstantiationPlan {
    pub fn from_scene(scene: AssetId, asset: &SceneAsset) -> Self {
        Self {
            scene,
            entity_count: asset.entities.len(),
            component_count: asset
                .entities
                .iter()
                .map(|entity| entity.components.len())
                .sum(),
            dependency_count: asset.dependencies.len(),
        }
    }

    pub fn commands(&self, asset: &SceneAsset) -> Vec<SceneInstantiationCommand> {
        let mut commands = Vec::with_capacity(self.entity_count + self.component_count);
        for (entity_index, entity) in asset.entities.iter().enumerate() {
            push_serialized_entity_commands(
                &mut commands,
                entity_index,
                entity,
                |entity_index, name, parent| SceneInstantiationCommand::SpawnEntity {
                    entity_index,
                    name,
                    parent,
                },
                |entity_index, type_name, data| SceneInstantiationCommand::AttachComponent {
                    entity_index,
                    type_name,
                    data,
                },
            );
        }
        commands
    }

    pub fn apply(&self, asset: &SceneAsset, sink: &mut impl InstantiationSink) {
        for (entity_index, entity) in asset.entities.iter().enumerate() {
            SceneInstantiationCommand::SpawnEntity {
                entity_index,
                name: entity.name.clone(),
                parent: entity.parent,
            }
            .apply(sink);
            for component in &entity.components {
                SceneInstantiationCommand::AttachComponent {
                    entity_index,
                    type_name: component.type_name.clone(),
                    data: component.data.clone(),
                }
                .apply(sink);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneInstantiationCommand {
    SpawnEntity {
        entity_index: usize,
        name: Option<String>,
        parent: Option<u64>,
    },
    AttachComponent {
        entity_index: usize,
        type_name: String,
        data: Vec<u8>,
    },
}

impl SceneInstantiationCommand {
    pub fn apply(&self, sink: &mut impl InstantiationSink) {
        match self {
            Self::SpawnEntity {
                entity_index,
                name,
                parent,
            } => sink.spawn_entity(*entity_index, name.as_deref(), *parent),
            Self::AttachComponent {
                entity_index,
                type_name,
                data,
            } => sink.attach_component(*entity_index, type_name, data),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PrefabInstanceComponent {
    pub prefab: Handle<Prefab>,
    pub loaded: bool,
}

impl PrefabInstanceComponent {
    pub fn asset_handles(&self) -> Vec<UntypedHandle> {
        vec![self.prefab.untyped()]
    }

    pub fn is_prefab_asset_ready(&self, assets: &AssetServer) -> bool {
        assets.is_ready_with_dependencies(&self.prefab)
    }

    pub fn can_instantiate(&self, assets: &AssetServer) -> bool {
        !self.loaded && self.is_prefab_asset_ready(assets)
    }

    pub fn instantiation_plan(&self, assets: &AssetServer) -> Option<PrefabInstantiationPlan> {
        if !self.can_instantiate(assets) {
            return None;
        }
        let prefab = assets.get(&self.prefab)?;
        Some(PrefabInstantiationPlan::from_prefab(
            self.prefab.id(),
            prefab,
        ))
    }

    pub fn instantiation_commands(
        &self,
        assets: &AssetServer,
    ) -> Option<Vec<PrefabInstantiationCommand>> {
        if !self.can_instantiate(assets) {
            return None;
        }
        let prefab = assets.get(&self.prefab)?;
        Some(PrefabInstantiationPlan::from_prefab(self.prefab.id(), prefab).commands(prefab))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefabInstantiationPlan {
    pub prefab: AssetId,
    pub entity_count: usize,
    pub component_count: usize,
    pub dependency_count: usize,
}

impl PrefabInstantiationPlan {
    pub fn from_prefab(prefab: AssetId, asset: &Prefab) -> Self {
        Self {
            prefab,
            entity_count: 1 + asset.children.len(),
            component_count: asset.root.components.len()
                + asset
                    .children
                    .iter()
                    .map(|entity| entity.components.len())
                    .sum::<usize>(),
            dependency_count: asset.dependencies.len(),
        }
    }

    pub fn commands(&self, asset: &Prefab) -> Vec<PrefabInstantiationCommand> {
        let mut commands = Vec::with_capacity(self.entity_count + self.component_count);
        push_serialized_entity_commands(
            &mut commands,
            0,
            &asset.root,
            |entity_index, name, parent| PrefabInstantiationCommand::SpawnEntity {
                entity_index,
                name,
                parent,
            },
            |entity_index, type_name, data| PrefabInstantiationCommand::AttachComponent {
                entity_index,
                type_name,
                data,
            },
        );
        for (child_index, child) in asset.children.iter().enumerate() {
            push_serialized_entity_commands(
                &mut commands,
                child_index + 1,
                child,
                |entity_index, name, parent| PrefabInstantiationCommand::SpawnEntity {
                    entity_index,
                    name,
                    parent,
                },
                |entity_index, type_name, data| PrefabInstantiationCommand::AttachComponent {
                    entity_index,
                    type_name,
                    data,
                },
            );
        }
        commands
    }

    pub fn apply(&self, asset: &Prefab, sink: &mut impl InstantiationSink) {
        PrefabInstantiationCommand::SpawnEntity {
            entity_index: 0,
            name: asset.root.name.clone(),
            parent: asset.root.parent,
        }
        .apply(sink);
        for component in &asset.root.components {
            PrefabInstantiationCommand::AttachComponent {
                entity_index: 0,
                type_name: component.type_name.clone(),
                data: component.data.clone(),
            }
            .apply(sink);
        }
        for (child_index, child) in asset.children.iter().enumerate() {
            let entity_index = child_index + 1;
            PrefabInstantiationCommand::SpawnEntity {
                entity_index,
                name: child.name.clone(),
                parent: child.parent,
            }
            .apply(sink);
            for component in &child.components {
                PrefabInstantiationCommand::AttachComponent {
                    entity_index,
                    type_name: component.type_name.clone(),
                    data: component.data.clone(),
                }
                .apply(sink);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrefabInstantiationCommand {
    SpawnEntity {
        entity_index: usize,
        name: Option<String>,
        parent: Option<u64>,
    },
    AttachComponent {
        entity_index: usize,
        type_name: String,
        data: Vec<u8>,
    },
}

impl PrefabInstantiationCommand {
    pub fn apply(&self, sink: &mut impl InstantiationSink) {
        match self {
            Self::SpawnEntity {
                entity_index,
                name,
                parent,
            } => sink.spawn_entity(*entity_index, name.as_deref(), *parent),
            Self::AttachComponent {
                entity_index,
                type_name,
                data,
            } => sink.attach_component(*entity_index, type_name, data),
        }
    }
}

fn push_serialized_entity_commands<C, F>(
    commands: &mut Vec<C>,
    entity_index: usize,
    entity: &crate::assets::scene::SerializedEntity,
    spawn_entity: F,
    attach_component: fn(usize, String, Vec<u8>) -> C,
) where
    F: Fn(usize, Option<String>, Option<u64>) -> C,
{
    commands.push(spawn_entity(
        entity_index,
        entity.name.clone(),
        entity.parent,
    ));
    for component in &entity.components {
        commands.push(attach_component(
            entity_index,
            component.type_name.clone(),
            component.data.clone(),
        ));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetSystemStage {
    AssetRequest,
    AssetServerUpdate,
    GpuUploadPrepare,
    AssetEventDispatch,
    SceneInstantiation,
    RenderPrepare,
    AudioPrepare,
    AssetGc,
}

impl AssetSystemStage {
    pub const fn order_index(self) -> usize {
        match self {
            Self::AssetRequest => 0,
            Self::AssetServerUpdate => 1,
            Self::GpuUploadPrepare => 2,
            Self::AssetEventDispatch => 3,
            Self::SceneInstantiation => 4,
            Self::RenderPrepare => 5,
            Self::AudioPrepare => 6,
            Self::AssetGc => 7,
        }
    }

    pub const fn system_name(self) -> &'static str {
        match self {
            Self::AssetRequest => "AssetRequestSystem",
            Self::AssetServerUpdate => "AssetServerUpdateSystem",
            Self::GpuUploadPrepare => "GpuUploadPrepareSystem",
            Self::AssetEventDispatch => "AssetEventDispatchSystem",
            Self::SceneInstantiation => "SceneInstantiationSystem",
            Self::RenderPrepare => "RenderPrepareSystem",
            Self::AudioPrepare => "AudioPrepareSystem",
            Self::AssetGc => "AssetGcSystem",
        }
    }

    pub const fn responsibility(self) -> &'static str {
        match self {
            Self::AssetRequest => "collect gameplay and scene asset load requests",
            Self::AssetServerUpdate => "advance asset loading, reload, events, and state",
            Self::GpuUploadPrepare => "drain GPU upload commands for renderer execution",
            Self::AssetEventDispatch => "dispatch ready, failed, reloaded, and unload events",
            Self::SceneInstantiation => "instantiate ready scene assets into the host ECS",
            Self::RenderPrepare => "prepare renderer inputs from ready asset handles",
            Self::AudioPrepare => "prepare audio inputs from ready asset handles",
            Self::AssetGc => "run asset garbage collection after consumers have prepared",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetSystemDescriptor {
    pub stage: AssetSystemStage,
    pub name: &'static str,
    pub responsibility: &'static str,
}

pub const ASSET_SYSTEM_ORDER: [AssetSystemStage; 8] = [
    AssetSystemStage::AssetRequest,
    AssetSystemStage::AssetServerUpdate,
    AssetSystemStage::GpuUploadPrepare,
    AssetSystemStage::AssetEventDispatch,
    AssetSystemStage::SceneInstantiation,
    AssetSystemStage::RenderPrepare,
    AssetSystemStage::AudioPrepare,
    AssetSystemStage::AssetGc,
];

pub fn asset_system_descriptors() -> Vec<AssetSystemDescriptor> {
    ASSET_SYSTEM_ORDER
        .iter()
        .copied()
        .map(|stage| AssetSystemDescriptor {
            stage,
            name: stage.system_name(),
            responsibility: stage.responsibility(),
        })
        .collect()
}

pub fn validate_asset_system_order(stages: &[AssetSystemStage]) -> bool {
    if stages.len() != ASSET_SYSTEM_ORDER.len() {
        return false;
    }
    stages
        .iter()
        .copied()
        .zip(ASSET_SYSTEM_ORDER)
        .all(|(actual, expected)| actual == expected)
}

pub fn stage_runs_before(left: AssetSystemStage, right: AssetSystemStage) -> bool {
    left.order_index() < right.order_index()
}
