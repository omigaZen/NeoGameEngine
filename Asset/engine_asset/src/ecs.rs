use crate::assets::{AudioClip, Material, Mesh, PhysicsMesh, SceneAsset, Skeleton};
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
