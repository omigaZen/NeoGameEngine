pub mod camera;
pub mod gltf;
pub mod light;
pub mod material;
pub mod mesh;
pub mod queue;
pub mod scene;
pub mod texture;
pub mod transform;

use engine_graphics::{Color, GraphicsResult, RenderSurface};

pub use camera::{Camera, OrthographicCamera, PerspectiveCamera, ViewCamera, ViewCameraProjection};
pub use gltf::{
    GltfAnimation, GltfAnimationChannel, GltfAnimationInterpolation, GltfAnimationLayer,
    GltfAnimationMixer, GltfAnimationOutput, GltfAnimationPath, GltfAnimationSample,
    GltfAnimationSampler, GltfAnimationValue, GltfAsset, GltfCamera, GltfCameraProjection,
    GltfImageData, GltfLoadError, GltfMaterial, GltfPrimitive, GltfPunctualLight,
    GltfPunctualLightKind,
};
pub use light::{
    DirectionalLight, DirectionalShadow, EnvironmentLight, PointLight, PointShadow, RenderLighting,
    SpotLight, SpotShadow, MAX_DIRECTIONAL_SHADOW_CASCADES, MAX_POINT_LIGHTS, MAX_SPOT_LIGHTS,
};
pub use material::{
    BlendMode, Material, MaterialLibrary, MaterialLoadError, MaterialTextureSamplers,
    NamedMaterial, TextureAddressMode, TextureFilterMode, TextureSampler, TextureTransform,
};
pub use mesh::{ColoredVertex, Mesh, MeshBounds, MeshLoadError, ObjMaterialMesh};
pub use queue::{
    RenderBatch, RenderDepthDesc, RenderItem, RenderPassDesc, RenderPassKind, RenderQueue,
    RenderQueueStats,
};
pub use scene::{
    GltfSceneLoadError, ImportedGltfPart, ImportedObjPart, MaterialHandle, MeshHandle,
    MeshInstance, MeshInstanceHandle, ObjSceneLoadError, RenderScene,
};
pub use texture::{Texture, TextureHandle, TextureLoadError, TextureSize};
pub use transform::{Mat4, Transform};

pub struct ClearRenderer {
    clear_color: Color,
}

impl ClearRenderer {
    pub fn new(clear_color: Color) -> Self {
        Self { clear_color }
    }

    pub fn clear_color(&self) -> Color {
        self.clear_color
    }

    pub fn set_clear_color(&mut self, clear_color: Color) {
        self.clear_color = clear_color;
    }

    pub fn render(&mut self, surface: &mut dyn RenderSurface) -> GraphicsResult<()> {
        surface.clear(self.clear_color)
    }
}
