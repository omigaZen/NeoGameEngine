use crate::asset::Asset;
use crate::assets::{
    serialized_component_asset_references, AudioClip, Material, Mesh, PhysicsMesh, Prefab,
    SceneAsset, SerializedComponent, SerializedEntity, Skeleton,
};
use crate::error::AssetError;
use crate::handle::{Handle, UntypedHandle};
use crate::id::{AssetId, AssetTypeId};
use crate::path::AssetPath;
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

    pub fn instantiate(&self, assets: &AssetServer, sink: &mut impl InstantiationSink) -> bool {
        if !self.can_instantiate(assets) {
            return false;
        }
        let Some(scene_asset) = assets.get(&self.scene) else {
            return false;
        };
        let plan = SceneInstantiationPlan::from_scene(self.scene.id(), scene_asset);
        plan.apply(scene_asset, sink);
        true
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

    pub fn instantiation_asset_references(
        &self,
        assets: &AssetServer,
    ) -> Result<Option<Vec<InstantiationAssetReference>>, AssetError> {
        if !self.can_instantiate(assets) {
            return Ok(None);
        }
        let Some(scene) = assets.get(&self.scene) else {
            return Ok(None);
        };
        SceneInstantiationPlan::from_scene(self.scene.id(), scene)
            .asset_references(scene)
            .map(Some)
    }

    pub fn instantiate_host<S: HostInstantiationSink>(
        &mut self,
        assets: &AssetServer,
        sink: &mut S,
    ) -> Result<Option<HostInstantiationReport<S::Entity>>, HostInstantiationError<S::Error>> {
        if !self.can_instantiate(assets) {
            return Ok(None);
        }
        let Some(scene_asset) = assets.get(&self.scene) else {
            return Ok(None);
        };
        let plan = SceneInstantiationPlan::from_scene(self.scene.id(), scene_asset);
        let report = plan.instantiate_host(scene_asset, sink)?;
        self.loaded = true;
        Ok(Some(report))
    }

    pub fn instantiate_typed_host<S: TypedHostInstantiationSink>(
        &mut self,
        assets: &mut AssetServer,
        sink: &mut S,
    ) -> Result<Option<HostInstantiationReport<S::Entity>>, TypedHostInstantiationError<S::Error>>
    {
        if !self.can_instantiate(assets) {
            return Ok(None);
        }
        let Some(scene_asset) = assets.get(&self.scene).cloned() else {
            return Ok(None);
        };
        let plan = SceneInstantiationPlan::from_scene(self.scene.id(), &scene_asset);
        let report = plan.instantiate_typed_host(&scene_asset, assets, sink)?;
        self.loaded = true;
        Ok(Some(report))
    }
}

pub trait InstantiationSink {
    fn spawn_entity(&mut self, entity_index: usize, name: Option<&str>, parent: Option<u64>);
    fn attach_component(&mut self, entity_index: usize, type_name: &str, data: &[u8]);
}

pub trait HostInstantiationSink {
    type Entity: Clone;
    type Error;

    fn spawn_entity(
        &mut self,
        entity_index: usize,
        name: Option<&str>,
        parent: Option<&Self::Entity>,
    ) -> Result<Self::Entity, Self::Error>;

    fn attach_component(
        &mut self,
        entity: &Self::Entity,
        entity_index: usize,
        component_index: usize,
        type_name: &str,
        data: &[u8],
    ) -> Result<(), Self::Error>;
}

pub trait TypedHostInstantiationSink {
    type Entity: Clone;
    type Error;

    fn spawn_entity(
        &mut self,
        entity_index: usize,
        name: Option<&str>,
        parent: Option<&Self::Entity>,
    ) -> Result<Self::Entity, Self::Error>;

    fn attach_component(
        &mut self,
        entity: &Self::Entity,
        entity_index: usize,
        component_index: usize,
        component: EcsComponentInstance,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug)]
pub enum EcsComponentInstance {
    MeshRenderer(MeshRendererComponent),
    SkinnedMeshRenderer(SkinnedMeshRendererComponent),
    AudioSource(AudioSourceComponent),
    PhysicsCollider(PhysicsColliderComponent),
    SceneInstance(SceneInstanceComponent),
    PrefabInstance(PrefabInstanceComponent),
    Unknown { type_name: String, data: Vec<u8> },
}

impl EcsComponentInstance {
    pub fn type_name(&self) -> &str {
        match self {
            Self::MeshRenderer(_) => "MeshRenderer",
            Self::SkinnedMeshRenderer(_) => "SkinnedMeshRenderer",
            Self::AudioSource(_) => "AudioSource",
            Self::PhysicsCollider(_) => "PhysicsCollider",
            Self::SceneInstance(_) => "SceneInstance",
            Self::PrefabInstance(_) => "PrefabInstance",
            Self::Unknown { type_name, .. } => type_name,
        }
    }

    pub fn asset_handles(&self) -> Vec<UntypedHandle> {
        match self {
            Self::MeshRenderer(component) => component.asset_handles(),
            Self::SkinnedMeshRenderer(component) => component.asset_handles(),
            Self::AudioSource(component) => component.asset_handles(),
            Self::PhysicsCollider(component) => component.asset_handles(),
            Self::SceneInstance(component) => component.asset_handles(),
            Self::PrefabInstance(component) => component.asset_handles(),
            Self::Unknown { .. } => Vec::new(),
        }
    }

    pub fn is_ready(&self, assets: &AssetServer) -> bool {
        match self {
            Self::MeshRenderer(component) => component.is_ready(assets),
            Self::SkinnedMeshRenderer(component) => component.is_ready(assets),
            Self::AudioSource(component) => component.is_ready(assets),
            Self::PhysicsCollider(component) => component.is_ready(assets),
            Self::SceneInstance(component) => component.is_scene_asset_ready(assets),
            Self::PrefabInstance(component) => component.is_prefab_asset_ready(assets),
            Self::Unknown { .. } => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EcsComponentMaterializationError {
    InvalidUtf8 {
        component_type: String,
        message: String,
    },
    InvalidField {
        component_type: String,
        field: String,
        message: String,
    },
    MissingField {
        component_type: String,
        field: String,
    },
    DuplicateField {
        component_type: String,
        field: String,
    },
    AssetReference {
        component_type: String,
        error: AssetError,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostInstantiationReport<Entity> {
    pub source: AssetId,
    pub entities: Vec<Entity>,
    pub root_entities: Vec<Entity>,
    pub attached_component_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostInstantiationError<E> {
    MissingParent {
        entity_index: usize,
        parent_index: u64,
    },
    Sink(E),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedHostInstantiationError<E> {
    MissingParent {
        entity_index: usize,
        parent_index: u64,
    },
    Component {
        entity_index: usize,
        component_index: usize,
        error: EcsComponentMaterializationError,
    },
    Sink(E),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstantiationAssetReference {
    pub entity_index: usize,
    pub component_index: usize,
    pub component_type: String,
    pub field: String,
    pub path: AssetPath,
    pub asset_type: AssetTypeId,
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

    pub fn asset_references(
        &self,
        asset: &SceneAsset,
    ) -> Result<Vec<InstantiationAssetReference>, AssetError> {
        let mut references = Vec::new();
        for (entity_index, entity) in asset.entities.iter().enumerate() {
            push_serialized_entity_asset_references(&mut references, entity_index, entity)?;
        }
        Ok(references)
    }

    pub fn instantiate_host<S: HostInstantiationSink>(
        &self,
        asset: &SceneAsset,
        sink: &mut S,
    ) -> Result<HostInstantiationReport<S::Entity>, HostInstantiationError<S::Error>> {
        instantiate_serialized_entities_with_host(self.scene, &asset.entities, sink)
    }

    pub fn instantiate_typed_host<S: TypedHostInstantiationSink>(
        &self,
        asset: &SceneAsset,
        assets: &mut AssetServer,
        sink: &mut S,
    ) -> Result<HostInstantiationReport<S::Entity>, TypedHostInstantiationError<S::Error>> {
        instantiate_serialized_entities_with_typed_host(self.scene, &asset.entities, assets, sink)
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

    pub fn instantiate(&self, assets: &AssetServer, sink: &mut impl InstantiationSink) -> bool {
        if !self.can_instantiate(assets) {
            return false;
        }
        let Some(prefab_asset) = assets.get(&self.prefab) else {
            return false;
        };
        let plan = PrefabInstantiationPlan::from_prefab(self.prefab.id(), prefab_asset);
        plan.apply(prefab_asset, sink);
        true
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

    pub fn instantiation_asset_references(
        &self,
        assets: &AssetServer,
    ) -> Result<Option<Vec<InstantiationAssetReference>>, AssetError> {
        if !self.can_instantiate(assets) {
            return Ok(None);
        }
        let Some(prefab) = assets.get(&self.prefab) else {
            return Ok(None);
        };
        PrefabInstantiationPlan::from_prefab(self.prefab.id(), prefab)
            .asset_references(prefab)
            .map(Some)
    }

    pub fn instantiate_host<S: HostInstantiationSink>(
        &mut self,
        assets: &AssetServer,
        sink: &mut S,
    ) -> Result<Option<HostInstantiationReport<S::Entity>>, HostInstantiationError<S::Error>> {
        if !self.can_instantiate(assets) {
            return Ok(None);
        }
        let Some(prefab_asset) = assets.get(&self.prefab) else {
            return Ok(None);
        };
        let plan = PrefabInstantiationPlan::from_prefab(self.prefab.id(), prefab_asset);
        let report = plan.instantiate_host(prefab_asset, sink)?;
        self.loaded = true;
        Ok(Some(report))
    }

    pub fn instantiate_typed_host<S: TypedHostInstantiationSink>(
        &mut self,
        assets: &mut AssetServer,
        sink: &mut S,
    ) -> Result<Option<HostInstantiationReport<S::Entity>>, TypedHostInstantiationError<S::Error>>
    {
        if !self.can_instantiate(assets) {
            return Ok(None);
        }
        let Some(prefab_asset) = assets.get(&self.prefab).cloned() else {
            return Ok(None);
        };
        let plan = PrefabInstantiationPlan::from_prefab(self.prefab.id(), &prefab_asset);
        let report = plan.instantiate_typed_host(&prefab_asset, assets, sink)?;
        self.loaded = true;
        Ok(Some(report))
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

    pub fn asset_references(
        &self,
        asset: &Prefab,
    ) -> Result<Vec<InstantiationAssetReference>, AssetError> {
        let mut references = Vec::new();
        push_serialized_entity_asset_references(&mut references, 0, &asset.root)?;
        for (child_index, child) in asset.children.iter().enumerate() {
            push_serialized_entity_asset_references(&mut references, child_index + 1, child)?;
        }
        Ok(references)
    }

    pub fn instantiate_host<S: HostInstantiationSink>(
        &self,
        asset: &Prefab,
        sink: &mut S,
    ) -> Result<HostInstantiationReport<S::Entity>, HostInstantiationError<S::Error>> {
        let mut entities = Vec::with_capacity(1 + asset.children.len());
        entities.push(&asset.root);
        entities.extend(asset.children.iter());
        instantiate_serialized_entity_refs_with_host(self.prefab, &entities, sink)
    }

    pub fn instantiate_typed_host<S: TypedHostInstantiationSink>(
        &self,
        asset: &Prefab,
        assets: &mut AssetServer,
        sink: &mut S,
    ) -> Result<HostInstantiationReport<S::Entity>, TypedHostInstantiationError<S::Error>> {
        let mut entities = Vec::with_capacity(1 + asset.children.len());
        entities.push(&asset.root);
        entities.extend(asset.children.iter());
        instantiate_serialized_entity_refs_with_typed_host(self.prefab, &entities, assets, sink)
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

fn instantiate_serialized_entities_with_host<S: HostInstantiationSink>(
    source: AssetId,
    entities: &[crate::assets::scene::SerializedEntity],
    sink: &mut S,
) -> Result<HostInstantiationReport<S::Entity>, HostInstantiationError<S::Error>> {
    let entity_refs = entities.iter().collect::<Vec<_>>();
    instantiate_serialized_entity_refs_with_host(source, &entity_refs, sink)
}

fn instantiate_serialized_entity_refs_with_host<S: HostInstantiationSink>(
    source: AssetId,
    entities: &[&crate::assets::scene::SerializedEntity],
    sink: &mut S,
) -> Result<HostInstantiationReport<S::Entity>, HostInstantiationError<S::Error>> {
    let mut spawned_entities = Vec::with_capacity(entities.len());
    let mut root_entities = Vec::new();
    let mut attached_component_count = 0;
    for (entity_index, entity) in entities.iter().enumerate() {
        let parent = match entity.parent {
            Some(parent_index) => {
                let Some(parent_index_usize) = usize::try_from(parent_index).ok() else {
                    return Err(HostInstantiationError::MissingParent {
                        entity_index,
                        parent_index,
                    });
                };
                Some(spawned_entities.get(parent_index_usize).ok_or(
                    HostInstantiationError::MissingParent {
                        entity_index,
                        parent_index,
                    },
                )?)
            }
            None => None,
        };
        let entity_handle = sink
            .spawn_entity(entity_index, entity.name.as_deref(), parent)
            .map_err(HostInstantiationError::Sink)?;
        if entity.parent.is_none() {
            root_entities.push(entity_handle.clone());
        }
        for (component_index, component) in entity.components.iter().enumerate() {
            sink.attach_component(
                &entity_handle,
                entity_index,
                component_index,
                &component.type_name,
                &component.data,
            )
            .map_err(HostInstantiationError::Sink)?;
            attached_component_count += 1;
        }
        spawned_entities.push(entity_handle);
    }
    Ok(HostInstantiationReport {
        source,
        entities: spawned_entities,
        root_entities,
        attached_component_count,
    })
}

fn instantiate_serialized_entities_with_typed_host<S: TypedHostInstantiationSink>(
    source: AssetId,
    entities: &[SerializedEntity],
    assets: &mut AssetServer,
    sink: &mut S,
) -> Result<HostInstantiationReport<S::Entity>, TypedHostInstantiationError<S::Error>> {
    let entity_refs = entities.iter().collect::<Vec<_>>();
    instantiate_serialized_entity_refs_with_typed_host(source, &entity_refs, assets, sink)
}

fn instantiate_serialized_entity_refs_with_typed_host<S: TypedHostInstantiationSink>(
    source: AssetId,
    entities: &[&SerializedEntity],
    assets: &mut AssetServer,
    sink: &mut S,
) -> Result<HostInstantiationReport<S::Entity>, TypedHostInstantiationError<S::Error>> {
    validate_serialized_entity_parent_indexes(entities)?;
    let materialized_components = entities
        .iter()
        .enumerate()
        .map(|(entity_index, entity)| {
            entity
                .components
                .iter()
                .enumerate()
                .map(|(component_index, component)| {
                    materialize_serialized_component(assets, component).map_err(|error| {
                        TypedHostInstantiationError::Component {
                            entity_index,
                            component_index,
                            error,
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut spawned_entities = Vec::with_capacity(entities.len());
    let mut root_entities = Vec::new();
    let mut attached_component_count = 0;
    for (entity_index, (entity, components)) in
        entities.iter().zip(materialized_components).enumerate()
    {
        let parent = match entity.parent {
            Some(parent_index) => Some(
                &spawned_entities[usize::try_from(parent_index).expect("parent index validated")],
            ),
            None => None,
        };
        let entity_handle = sink
            .spawn_entity(entity_index, entity.name.as_deref(), parent)
            .map_err(TypedHostInstantiationError::Sink)?;
        if entity.parent.is_none() {
            root_entities.push(entity_handle.clone());
        }
        for (component_index, component_instance) in components.into_iter().enumerate() {
            sink.attach_component(
                &entity_handle,
                entity_index,
                component_index,
                component_instance,
            )
            .map_err(TypedHostInstantiationError::Sink)?;
            attached_component_count += 1;
        }
        spawned_entities.push(entity_handle);
    }
    Ok(HostInstantiationReport {
        source,
        entities: spawned_entities,
        root_entities,
        attached_component_count,
    })
}

fn validate_serialized_entity_parent_indexes<E>(
    entities: &[&SerializedEntity],
) -> Result<(), TypedHostInstantiationError<E>> {
    for (entity_index, entity) in entities.iter().enumerate() {
        let Some(parent_index) = entity.parent else {
            continue;
        };
        let Some(parent_index_usize) = usize::try_from(parent_index).ok() else {
            return Err(TypedHostInstantiationError::MissingParent {
                entity_index,
                parent_index,
            });
        };
        if parent_index_usize >= entity_index {
            return Err(TypedHostInstantiationError::MissingParent {
                entity_index,
                parent_index,
            });
        }
    }
    Ok(())
}

pub fn materialize_serialized_component(
    assets: &mut AssetServer,
    component: &SerializedComponent,
) -> Result<EcsComponentInstance, EcsComponentMaterializationError> {
    let component_key = component.type_name.to_ascii_lowercase();
    if !matches!(
        component_key.as_str(),
        "meshrenderer"
            | "skinnedmeshrenderer"
            | "audiosource"
            | "physicscollider"
            | "sceneinstance"
            | "prefabinstance"
    ) {
        return Ok(EcsComponentInstance::Unknown {
            type_name: component.type_name.clone(),
            data: component.data.clone(),
        });
    }

    let fields = parse_serialized_component_fields(component)?;
    validate_serialized_component_asset_fields(component)?;
    match component_key.as_str() {
        "meshrenderer" => Ok(EcsComponentInstance::MeshRenderer(MeshRendererComponent {
            mesh: required_asset_handle::<Mesh>(assets, &fields, &component.type_name, "mesh")?,
            material: required_asset_handle::<Material>(
                assets,
                &fields,
                &component.type_name,
                "material",
            )?,
        })),
        "skinnedmeshrenderer" => Ok(EcsComponentInstance::SkinnedMeshRenderer(
            SkinnedMeshRendererComponent {
                mesh: required_asset_handle::<Mesh>(assets, &fields, &component.type_name, "mesh")?,
                skeleton: required_asset_handle::<Skeleton>(
                    assets,
                    &fields,
                    &component.type_name,
                    "skeleton",
                )?,
                material: required_asset_handle::<Material>(
                    assets,
                    &fields,
                    &component.type_name,
                    "material",
                )?,
            },
        )),
        "audiosource" => Ok(EcsComponentInstance::AudioSource(AudioSourceComponent {
            clip: required_asset_handle::<AudioClip>(
                assets,
                &fields,
                &component.type_name,
                "clip",
            )?,
            looping: optional_component_bool(&fields, &component.type_name, "looping", false)?,
            volume: optional_component_f32(&fields, &component.type_name, "volume", 1.0, 0.0)?,
        })),
        "physicscollider" => Ok(EcsComponentInstance::PhysicsCollider(
            PhysicsColliderComponent {
                mesh: required_asset_handle_any::<PhysicsMesh>(
                    assets,
                    &fields,
                    &component.type_name,
                    &["mesh", "physics_mesh"],
                )?,
                dynamic: optional_component_bool(&fields, &component.type_name, "dynamic", false)?,
            },
        )),
        "sceneinstance" => Ok(EcsComponentInstance::SceneInstance(
            SceneInstanceComponent {
                scene: required_asset_handle::<SceneAsset>(
                    assets,
                    &fields,
                    &component.type_name,
                    "scene",
                )?,
                loaded: optional_component_bool(&fields, &component.type_name, "loaded", false)?,
            },
        )),
        "prefabinstance" => Ok(EcsComponentInstance::PrefabInstance(
            PrefabInstanceComponent {
                prefab: required_asset_handle::<Prefab>(
                    assets,
                    &fields,
                    &component.type_name,
                    "prefab",
                )?,
                loaded: optional_component_bool(&fields, &component.type_name, "loaded", false)?,
            },
        )),
        _ => unreachable!("component key recognized above"),
    }
}

fn parse_serialized_component_fields(
    component: &SerializedComponent,
) -> Result<Vec<(String, String)>, EcsComponentMaterializationError> {
    let data = std::str::from_utf8(&component.data).map_err(|error| {
        EcsComponentMaterializationError::InvalidUtf8 {
            component_type: component.type_name.clone(),
            message: error.to_string(),
        }
    })?;
    let mut fields = Vec::new();
    for field in data
        .split(';')
        .map(str::trim)
        .filter(|field| !field.is_empty())
    {
        let Some((key, value)) = field.split_once('=') else {
            return Err(EcsComponentMaterializationError::InvalidField {
                component_type: component.type_name.clone(),
                field: field.to_owned(),
                message: "expected key=value".to_owned(),
            });
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(EcsComponentMaterializationError::InvalidField {
                component_type: component.type_name.clone(),
                field: field.to_owned(),
                message: "field name is empty".to_owned(),
            });
        }
        fields.push((key.to_owned(), value.trim().to_owned()));
    }
    Ok(fields)
}

fn validate_serialized_component_asset_fields(
    component: &SerializedComponent,
) -> Result<(), EcsComponentMaterializationError> {
    serialized_component_asset_references(component)
        .map(|_| ())
        .map_err(|error| EcsComponentMaterializationError::AssetReference {
            component_type: component.type_name.clone(),
            error,
        })
}

fn required_asset_handle<T: Asset>(
    assets: &mut AssetServer,
    fields: &[(String, String)],
    component_type: &str,
    field: &str,
) -> Result<Handle<T>, EcsComponentMaterializationError> {
    let path = required_component_path(fields, component_type, field)?;
    Ok(assets.load::<T>(path))
}

fn required_asset_handle_any<T: Asset>(
    assets: &mut AssetServer,
    fields: &[(String, String)],
    component_type: &str,
    field_names: &[&str],
) -> Result<Handle<T>, EcsComponentMaterializationError> {
    for field in field_names {
        if let Some(value) = component_field_value(fields, component_type, field)? {
            return Ok(assets.load::<T>(AssetPath::parse(value)));
        }
    }
    Err(EcsComponentMaterializationError::MissingField {
        component_type: component_type.to_owned(),
        field: field_names.join("|"),
    })
}

fn required_component_path(
    fields: &[(String, String)],
    component_type: &str,
    field: &str,
) -> Result<AssetPath, EcsComponentMaterializationError> {
    let value = component_field_value(fields, component_type, field)?.ok_or_else(|| {
        EcsComponentMaterializationError::MissingField {
            component_type: component_type.to_owned(),
            field: field.to_owned(),
        }
    })?;
    Ok(AssetPath::parse(value))
}

fn optional_component_bool(
    fields: &[(String, String)],
    component_type: &str,
    field: &str,
    default: bool,
) -> Result<bool, EcsComponentMaterializationError> {
    let Some(value) = component_field_value(fields, component_type, field)? else {
        return Ok(default);
    };
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(EcsComponentMaterializationError::InvalidField {
            component_type: component_type.to_owned(),
            field: field.to_owned(),
            message: format!("invalid bool value `{value}`"),
        }),
    }
}

fn optional_component_f32(
    fields: &[(String, String)],
    component_type: &str,
    field: &str,
    default: f32,
    min: f32,
) -> Result<f32, EcsComponentMaterializationError> {
    let Some(value) = component_field_value(fields, component_type, field)? else {
        return Ok(default);
    };
    let parsed =
        value
            .parse::<f32>()
            .map_err(|error| EcsComponentMaterializationError::InvalidField {
                component_type: component_type.to_owned(),
                field: field.to_owned(),
                message: format!("invalid float value `{value}`: {error}"),
            })?;
    if !parsed.is_finite() || parsed < min {
        return Err(EcsComponentMaterializationError::InvalidField {
            component_type: component_type.to_owned(),
            field: field.to_owned(),
            message: format!("value `{value}` must be finite and at least {min}"),
        });
    }
    Ok(parsed)
}

fn component_field_value<'a>(
    fields: &'a [(String, String)],
    component_type: &str,
    field: &str,
) -> Result<Option<&'a str>, EcsComponentMaterializationError> {
    let mut matches = fields
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case(field));
    let Some((_, value)) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(EcsComponentMaterializationError::DuplicateField {
            component_type: component_type.to_owned(),
            field: field.to_owned(),
        });
    }
    Ok(Some(value.as_str()))
}

fn push_serialized_entity_asset_references(
    references: &mut Vec<InstantiationAssetReference>,
    entity_index: usize,
    entity: &crate::assets::scene::SerializedEntity,
) -> Result<(), AssetError> {
    for (component_index, component) in entity.components.iter().enumerate() {
        for reference in serialized_component_asset_references(component)? {
            references.push(InstantiationAssetReference {
                entity_index,
                component_index,
                component_type: reference.component_type,
                field: reference.field,
                path: reference.path,
                asset_type: reference.asset_type,
            });
        }
    }
    Ok(())
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
