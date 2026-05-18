use std::fmt;

use engine_graphics::Color;

use crate::{
    Camera, GltfAnimationPath, GltfAnimationSample, GltfAnimationValue, GltfAsset, GltfImageData,
    GltfLoadError, GltfPrimitive, Mat4, Material, MaterialLibrary, MaterialLoadError, Mesh,
    MeshLoadError, RenderDepthDesc, RenderLighting, Texture, TextureHandle, TextureLoadError,
    Transform,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle {
    index: usize,
    generation: u32,
}

impl MeshHandle {
    const fn new(index: usize, generation: u32) -> Self {
        Self { index, generation }
    }

    pub const fn index(self) -> usize {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialHandle {
    index: usize,
    generation: u32,
}

impl MaterialHandle {
    const fn new(index: usize, generation: u32) -> Self {
        Self { index, generation }
    }

    pub const fn index(self) -> usize {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshInstanceHandle {
    index: usize,
    generation: u32,
}

impl MeshInstanceHandle {
    const fn new(index: usize, generation: u32) -> Self {
        Self { index, generation }
    }

    pub const fn index(self) -> usize {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshInstance {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub transform: Transform,
    pub visible: bool,
    pub sort_order: i32,
    model_matrix_override: Option<Mat4>,
}

impl MeshInstance {
    pub const fn new(mesh: MeshHandle, material: MaterialHandle, transform: Transform) -> Self {
        Self {
            mesh,
            material,
            transform,
            visible: true,
            sort_order: 0,
            model_matrix_override: None,
        }
    }

    pub const fn with_model_matrix(
        mesh: MeshHandle,
        material: MaterialHandle,
        transform: Transform,
        model_matrix: Mat4,
    ) -> Self {
        Self {
            mesh,
            material,
            transform,
            visible: true,
            sort_order: 0,
            model_matrix_override: Some(model_matrix),
        }
    }

    pub fn model_matrix(self) -> Mat4 {
        self.model_matrix_override
            .unwrap_or_else(|| self.transform.to_matrix())
    }

    pub fn normal_matrix(self) -> Mat4 {
        self.model_matrix_override
            .map(Mat4::normal_matrix)
            .unwrap_or_else(|| self.transform.normal_matrix())
    }

    pub fn sort_position(self) -> [f32; 3] {
        self.model_matrix().transform_point3([0.0, 0.0, 0.0])
    }

    pub fn model_matrix_override(self) -> Option<Mat4> {
        self.model_matrix_override
    }

    pub fn set_model_matrix_override(&mut self, model_matrix: Mat4) {
        self.model_matrix_override = Some(model_matrix);
    }

    pub fn clear_model_matrix_override(&mut self) {
        self.model_matrix_override = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedObjPart {
    pub material_name: Option<String>,
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub instance: MeshInstanceHandle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedGltfPart {
    pub node_index: Option<usize>,
    pub mesh_index: usize,
    pub primitive_index: usize,
    pub material_index: Option<usize>,
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub instance: MeshInstanceHandle,
    mesh_animation_source: Option<GltfPrimitive>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SampledNodeTransform {
    translation: Option<[f32; 3]>,
    rotation: Option<[f32; 4]>,
    scale: Option<[f32; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjSceneLoadError {
    Mesh(MeshLoadError),
    Material(MaterialLoadError),
    TextureNotFound { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GltfSceneLoadError {
    Gltf(GltfLoadError),
    Texture(TextureLoadError),
    TextureNotFound { path: String },
}

impl fmt::Display for GltfSceneLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gltf(error) => write!(f, "{error}"),
            Self::Texture(error) => write!(f, "{error}"),
            Self::TextureNotFound { path } => write!(f, "texture '{path}' was not resolved"),
        }
    }
}

impl std::error::Error for GltfSceneLoadError {}

impl From<GltfLoadError> for GltfSceneLoadError {
    fn from(error: GltfLoadError) -> Self {
        Self::Gltf(error)
    }
}

impl From<TextureLoadError> for GltfSceneLoadError {
    fn from(error: TextureLoadError) -> Self {
        Self::Texture(error)
    }
}

impl fmt::Display for ObjSceneLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mesh(error) => write!(f, "{error}"),
            Self::Material(error) => write!(f, "{error}"),
            Self::TextureNotFound { path } => write!(f, "texture '{path}' was not resolved"),
        }
    }
}

impl std::error::Error for ObjSceneLoadError {}

impl From<MeshLoadError> for ObjSceneLoadError {
    fn from(error: MeshLoadError) -> Self {
        Self::Mesh(error)
    }
}

impl From<MaterialLoadError> for ObjSceneLoadError {
    fn from(error: MaterialLoadError) -> Self {
        Self::Material(error)
    }
}

#[derive(Debug, Clone)]
struct SceneSlot<T> {
    generation: u32,
    revision: u64,
    value: Option<T>,
}

impl<T> SceneSlot<T> {
    fn occupied(value: T, revision: u64) -> Self {
        Self {
            generation: 0,
            revision,
            value: Some(value),
        }
    }

    fn handle_generation(&self) -> u32 {
        self.generation
    }

    fn is_occupied(&self) -> bool {
        self.value.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct RenderScene {
    meshes: Vec<SceneSlot<Mesh>>,
    free_meshes: Vec<usize>,
    textures: Vec<SceneSlot<Texture>>,
    free_textures: Vec<usize>,
    materials: Vec<SceneSlot<Material>>,
    free_materials: Vec<usize>,
    instances: Vec<SceneSlot<MeshInstance>>,
    free_instances: Vec<usize>,
    default_material: MaterialHandle,
    camera: Camera,
    clear_color: Color,
    depth: RenderDepthDesc,
    lighting: RenderLighting,
    aspect_ratio: f32,
    frustum_culling: bool,
    mesh_revision: u64,
    texture_revision: u64,
    material_revision: u64,
    instance_revision: u64,
}

impl RenderScene {
    pub fn new(camera: impl Into<Camera>) -> Self {
        let mut scene = Self {
            meshes: Vec::new(),
            free_meshes: Vec::new(),
            textures: Vec::new(),
            free_textures: Vec::new(),
            materials: Vec::new(),
            free_materials: Vec::new(),
            instances: Vec::new(),
            free_instances: Vec::new(),
            default_material: MaterialHandle::new(usize::MAX, 0),
            camera: camera.into(),
            clear_color: Color::rgb(0.05, 0.09, 0.13),
            depth: RenderDepthDesc::default(),
            lighting: RenderLighting::default(),
            aspect_ratio: 1.0,
            frustum_culling: false,
            mesh_revision: 0,
            texture_revision: 0,
            material_revision: 0,
            instance_revision: 0,
        };
        scene.default_material = scene.add_material(Material::default());
        scene
    }

    pub fn add_mesh(&mut self, mesh: Mesh) -> MeshHandle {
        let (index, generation) = insert_slot(
            &mut self.meshes,
            &mut self.free_meshes,
            mesh,
            &mut self.mesh_revision,
        );
        MeshHandle::new(index, generation)
    }

    pub fn replace_mesh(&mut self, handle: MeshHandle, mesh: Mesh) -> Option<()> {
        let slot = mesh_slot_mut(&mut self.meshes, handle)?;
        slot.value = Some(mesh);
        slot.revision = advance_revision(&mut self.mesh_revision);
        Some(())
    }

    pub fn remove_mesh(&mut self, handle: MeshHandle) -> Option<Mesh> {
        remove_slot(
            &mut self.meshes,
            &mut self.free_meshes,
            handle.index(),
            handle.generation(),
            &mut self.mesh_revision,
        )
    }

    pub fn add_texture(&mut self, texture: Texture) -> TextureHandle {
        let (index, generation) = insert_slot(
            &mut self.textures,
            &mut self.free_textures,
            texture,
            &mut self.texture_revision,
        );
        TextureHandle::new(index, generation)
    }

    pub fn replace_texture(&mut self, handle: TextureHandle, texture: Texture) -> Option<()> {
        let slot = texture_slot_mut(&mut self.textures, handle)?;
        slot.value = Some(texture);
        slot.revision = advance_revision(&mut self.texture_revision);
        Some(())
    }

    pub fn remove_texture(&mut self, handle: TextureHandle) -> Option<Texture> {
        remove_slot(
            &mut self.textures,
            &mut self.free_textures,
            handle.index(),
            handle.generation(),
            &mut self.texture_revision,
        )
    }

    pub fn add_material(&mut self, material: Material) -> MaterialHandle {
        let (index, generation) = insert_slot(
            &mut self.materials,
            &mut self.free_materials,
            material,
            &mut self.material_revision,
        );
        MaterialHandle::new(index, generation)
    }

    pub fn replace_material(&mut self, handle: MaterialHandle, material: Material) -> Option<()> {
        let slot = material_slot_mut(&mut self.materials, handle)?;
        slot.value = Some(material);
        slot.revision = advance_revision(&mut self.material_revision);
        Some(())
    }

    pub fn remove_material(&mut self, handle: MaterialHandle) -> Option<Material> {
        if handle == self.default_material {
            return None;
        }

        remove_slot(
            &mut self.materials,
            &mut self.free_materials,
            handle.index(),
            handle.generation(),
            &mut self.material_revision,
        )
    }

    pub fn default_material(&self) -> MaterialHandle {
        self.default_material
    }

    pub fn add_instance(&mut self, mesh: MeshHandle, transform: Transform) -> MeshInstanceHandle {
        self.add_instance_with_material(mesh, self.default_material(), transform)
    }

    pub fn add_instance_with_material(
        &mut self,
        mesh: MeshHandle,
        material: MaterialHandle,
        transform: Transform,
    ) -> MeshInstanceHandle {
        let instance = MeshInstance::new(mesh, material, transform);
        let (index, generation) = insert_slot(
            &mut self.instances,
            &mut self.free_instances,
            instance,
            &mut self.instance_revision,
        );
        MeshInstanceHandle::new(index, generation)
    }

    pub fn add_instance_with_material_matrix(
        &mut self,
        mesh: MeshHandle,
        material: MaterialHandle,
        model_matrix: Mat4,
    ) -> MeshInstanceHandle {
        let translation = model_matrix.transform_point3([0.0, 0.0, 0.0]);
        let transform = Transform::new_3d(translation, [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let instance = MeshInstance::with_model_matrix(mesh, material, transform, model_matrix);
        let (index, generation) = insert_slot(
            &mut self.instances,
            &mut self.free_instances,
            instance,
            &mut self.instance_revision,
        );
        MeshInstanceHandle::new(index, generation)
    }

    pub fn remove_instance(&mut self, handle: MeshInstanceHandle) -> Option<MeshInstance> {
        remove_slot(
            &mut self.instances,
            &mut self.free_instances,
            handle.index(),
            handle.generation(),
            &mut self.instance_revision,
        )
    }

    pub fn add_obj_mtl_instances(
        &mut self,
        obj_source: &str,
        mtl_source: &str,
        transform: Transform,
    ) -> Result<Vec<ImportedObjPart>, ObjSceneLoadError> {
        let materials = MaterialLibrary::from_mtl_str(mtl_source)?;
        self.add_obj_instances_with_material_library(obj_source, &materials, transform)
    }

    pub fn add_obj_mtl_instances_with_textures(
        &mut self,
        obj_source: &str,
        mtl_source: &str,
        transform: Transform,
        texture_resolver: impl FnMut(&str) -> Option<Texture>,
    ) -> Result<Vec<ImportedObjPart>, ObjSceneLoadError> {
        let materials = MaterialLibrary::from_mtl_str(mtl_source)?;
        self.add_obj_instances_with_material_library_and_textures(
            obj_source,
            &materials,
            transform,
            texture_resolver,
        )
    }

    pub fn add_obj_instances_with_material_library(
        &mut self,
        obj_source: &str,
        materials: &MaterialLibrary,
        transform: Transform,
    ) -> Result<Vec<ImportedObjPart>, ObjSceneLoadError> {
        let parts = Mesh::from_obj_str_by_material(obj_source, [1.0, 1.0, 1.0])?;
        let mut imported = Vec::with_capacity(parts.len());

        for part in parts {
            let material_name = part.material_name;
            let material = material_name
                .as_deref()
                .and_then(|name| materials.material(name))
                .map(|material| self.add_material(material))
                .unwrap_or_else(|| self.default_material());
            let mesh = self.add_mesh(part.mesh);
            let instance = self.add_instance_with_material(mesh, material, transform);

            imported.push(ImportedObjPart {
                material_name,
                mesh,
                material,
                instance,
            });
        }

        Ok(imported)
    }

    pub fn add_obj_instances_with_material_library_and_textures(
        &mut self,
        obj_source: &str,
        materials: &MaterialLibrary,
        transform: Transform,
        mut texture_resolver: impl FnMut(&str) -> Option<Texture>,
    ) -> Result<Vec<ImportedObjPart>, ObjSceneLoadError> {
        let parts = Mesh::from_obj_str_by_material(obj_source, [1.0, 1.0, 1.0])?;
        let mut imported = Vec::with_capacity(parts.len());

        for part in parts {
            let material_name = part.material_name;
            let material = match material_name
                .as_deref()
                .and_then(|name| materials.named_material(name))
            {
                Some(named_material) => {
                    let mut material = named_material.material;
                    if let Some(path) = &named_material.base_color_texture_path {
                        let texture = texture_resolver(path).ok_or_else(|| {
                            ObjSceneLoadError::TextureNotFound { path: path.clone() }
                        })?;
                        material.base_color_texture = Some(self.add_texture(texture));
                    }
                    self.add_material(material)
                }
                None => self.default_material(),
            };
            let mesh = self.add_mesh(part.mesh);
            let instance = self.add_instance_with_material(mesh, material, transform);

            imported.push(ImportedObjPart {
                material_name,
                mesh,
                material,
                instance,
            });
        }

        Ok(imported)
    }

    pub fn add_gltf_instances_with_buffers_and_textures(
        &mut self,
        gltf_source: &str,
        transform: Transform,
        buffer_resolver: impl FnMut(&str) -> Option<Vec<u8>>,
        texture_resolver: impl FnMut(&str) -> Option<Texture>,
    ) -> Result<Vec<ImportedGltfPart>, GltfSceneLoadError> {
        let asset = GltfAsset::from_gltf_str_with_buffers(gltf_source, buffer_resolver)?;
        self.add_gltf_asset_instances_with_textures(asset, transform, texture_resolver)
    }

    pub fn add_glb_instances_with_buffers_and_textures(
        &mut self,
        glb_bytes: &[u8],
        transform: Transform,
        buffer_resolver: impl FnMut(&str) -> Option<Vec<u8>>,
        texture_resolver: impl FnMut(&str) -> Option<Texture>,
    ) -> Result<Vec<ImportedGltfPart>, GltfSceneLoadError> {
        let asset = GltfAsset::from_glb_bytes_with_buffers(glb_bytes, buffer_resolver)?;
        self.add_gltf_asset_instances_with_textures(asset, transform, texture_resolver)
    }

    fn add_gltf_asset_instances_with_textures(
        &mut self,
        asset: GltfAsset,
        transform: Transform,
        mut texture_resolver: impl FnMut(&str) -> Option<Texture>,
    ) -> Result<Vec<ImportedGltfPart>, GltfSceneLoadError> {
        let imported_lighting = asset.punctual_lighting_with_transform(transform.to_matrix());
        let mut material_handles = Vec::with_capacity(asset.materials.len());
        for material in &asset.materials {
            let mut scene_material = material.material;
            scene_material.base_color_texture = load_gltf_texture(
                self,
                material.base_color_texture_path.as_deref(),
                material.base_color_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.metallic_roughness_texture = load_gltf_texture(
                self,
                material.metallic_roughness_texture_path.as_deref(),
                material.metallic_roughness_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.normal_texture = load_gltf_texture(
                self,
                material.normal_texture_path.as_deref(),
                material.normal_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.emissive_texture = load_gltf_texture(
                self,
                material.emissive_texture_path.as_deref(),
                material.emissive_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.occlusion_texture = load_gltf_texture(
                self,
                material.occlusion_texture_path.as_deref(),
                material.occlusion_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.clearcoat_texture = load_gltf_texture(
                self,
                material.clearcoat_texture_path.as_deref(),
                material.clearcoat_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.clearcoat_roughness_texture = load_gltf_texture(
                self,
                material.clearcoat_roughness_texture_path.as_deref(),
                material.clearcoat_roughness_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.clearcoat_normal_texture = load_gltf_texture(
                self,
                material.clearcoat_normal_texture_path.as_deref(),
                material.clearcoat_normal_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.sheen_color_texture = load_gltf_texture(
                self,
                material.sheen_color_texture_path.as_deref(),
                material.sheen_color_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.sheen_roughness_texture = load_gltf_texture(
                self,
                material.sheen_roughness_texture_path.as_deref(),
                material.sheen_roughness_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.transmission_texture = load_gltf_texture(
                self,
                material.transmission_texture_path.as_deref(),
                material.transmission_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.specular_texture = load_gltf_texture(
                self,
                material.specular_texture_path.as_deref(),
                material.specular_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.specular_color_texture = load_gltf_texture(
                self,
                material.specular_color_texture_path.as_deref(),
                material.specular_color_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.anisotropy_texture = load_gltf_texture(
                self,
                material.anisotropy_texture_path.as_deref(),
                material.anisotropy_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.iridescence_texture = load_gltf_texture(
                self,
                material.iridescence_texture_path.as_deref(),
                material.iridescence_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.iridescence_thickness_texture = load_gltf_texture(
                self,
                material.iridescence_thickness_texture_path.as_deref(),
                material.iridescence_thickness_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            scene_material.thickness_texture = load_gltf_texture(
                self,
                material.thickness_texture_path.as_deref(),
                material.thickness_texture_data.as_ref(),
                &mut texture_resolver,
            )?;
            material_handles.push(self.add_material(scene_material));
        }

        let mut imported = Vec::with_capacity(asset.primitives.len());
        for primitive in asset.primitives {
            let mesh_animation_source = (primitive.morph_target_count > 0
                || primitive.skin_joint_count > 0)
                .then(|| primitive.clone());
            let material = primitive
                .material_index
                .and_then(|index| material_handles.get(index).copied())
                .unwrap_or_else(|| self.default_material());
            let mesh = self.add_mesh(primitive.mesh);
            let instance = if primitive.model_matrix == Mat4::IDENTITY {
                self.add_instance_with_material(mesh, material, transform)
            } else {
                self.add_instance_with_material_matrix(
                    mesh,
                    material,
                    transform.to_matrix() * primitive.model_matrix,
                )
            };

            imported.push(ImportedGltfPart {
                node_index: primitive.node_index,
                mesh_index: primitive.mesh_index,
                primitive_index: primitive.primitive_index,
                material_index: primitive.material_index,
                mesh,
                material,
                instance,
                mesh_animation_source,
            });
        }

        if let Some(lighting) = imported_lighting {
            self.set_lighting(lighting);
        }

        Ok(imported)
    }

    pub fn apply_gltf_animation_samples_to_imported_parts(
        &mut self,
        imported: &[ImportedGltfPart],
        root_transform: Transform,
        samples: &[GltfAnimationSample],
    ) -> usize {
        let sampled_nodes = sampled_node_transforms(samples);
        let sampled_node_matrices = sampled_node_matrices(&sampled_nodes);
        let sampled_weights = sampled_node_weights(samples);
        let root_matrix = root_transform.to_matrix();
        let mut updated = 0;

        for part in imported {
            let Some(node_index) = part.node_index else {
                continue;
            };
            let mut part_updated = false;

            if let Some((_, sampled)) = sampled_nodes
                .iter()
                .find(|(sampled_node, _)| *sampled_node == node_index)
            {
                let translation = sampled.translation.unwrap_or([0.0, 0.0, 0.0]);
                let rotation = sampled.rotation.unwrap_or([0.0, 0.0, 0.0, 1.0]);
                let scale = sampled.scale.unwrap_or([1.0, 1.0, 1.0]);
                let model_matrix = root_matrix
                    * Mat4::translation(translation)
                    * Mat4::rotation_quaternion(rotation)
                    * Mat4::scale(scale);

                part_updated |= self
                    .set_instance_model_matrix(part.instance, model_matrix)
                    .is_some();
            }

            if let Some((_, weights)) = sampled_weights
                .iter()
                .find(|(sampled_node, _)| *sampled_node == node_index)
            {
                if let Some(source) = part.mesh_animation_source.as_ref() {
                    let mesh = source.animated_mesh_with_joint_world_matrices(
                        Some(weights),
                        &sampled_node_matrices,
                    );
                    part_updated |= self.replace_mesh(part.mesh, mesh).is_some();
                }
            } else if let Some(source) = part.mesh_animation_source.as_ref() {
                let skin_sampled = source.has_live_skinning_source()
                    && sampled_node_matrices
                        .iter()
                        .any(|(sampled_node, _)| source.is_skin_affected_by_node(*sampled_node));
                if skin_sampled {
                    let mesh = source
                        .animated_mesh_with_joint_world_matrices(None, &sampled_node_matrices);
                    part_updated |= self.replace_mesh(part.mesh, mesh).is_some();
                }
            }

            if part_updated {
                updated += 1;
            }
        }

        updated
    }

    pub fn mesh(&self, handle: MeshHandle) -> Option<&Mesh> {
        mesh_slot(&self.meshes, handle)?.value.as_ref()
    }

    pub fn material(&self, handle: MaterialHandle) -> Option<&Material> {
        material_slot(&self.materials, handle)?.value.as_ref()
    }

    pub fn texture(&self, handle: TextureHandle) -> Option<&Texture> {
        texture_slot(&self.textures, handle)?.value.as_ref()
    }

    pub fn instance(&self, handle: MeshInstanceHandle) -> Option<&MeshInstance> {
        instance_slot(&self.instances, handle)?.value.as_ref()
    }

    pub fn instance_mut(&mut self, handle: MeshInstanceHandle) -> Option<&mut MeshInstance> {
        let slot = instance_slot_mut(&mut self.instances, handle)?;
        advance_revision(&mut self.instance_revision);
        slot.value.as_mut()
    }

    pub fn set_instance_transform(
        &mut self,
        handle: MeshInstanceHandle,
        transform: Transform,
    ) -> Option<()> {
        let instance = instance_slot_mut(&mut self.instances, handle)?
            .value
            .as_mut()?;
        instance.transform = transform;
        instance.clear_model_matrix_override();
        Some(())
    }

    pub fn set_instance_model_matrix(
        &mut self,
        handle: MeshInstanceHandle,
        model_matrix: Mat4,
    ) -> Option<()> {
        let instance = instance_slot_mut(&mut self.instances, handle)?
            .value
            .as_mut()?;
        instance.transform = Transform::new_3d(
            model_matrix.transform_point3([0.0, 0.0, 0.0]),
            [0.0; 3],
            [1.0; 3],
        );
        instance.set_model_matrix_override(model_matrix);
        Some(())
    }

    pub fn set_instance_visible(
        &mut self,
        handle: MeshInstanceHandle,
        visible: bool,
    ) -> Option<()> {
        let instance = instance_slot_mut(&mut self.instances, handle)?
            .value
            .as_mut()?;
        instance.visible = visible;
        Some(())
    }

    pub fn set_instance_sort_order(
        &mut self,
        handle: MeshInstanceHandle,
        sort_order: i32,
    ) -> Option<()> {
        let instance = instance_slot_mut(&mut self.instances, handle)?
            .value
            .as_mut()?;
        instance.sort_order = sort_order;
        Some(())
    }

    pub fn set_instance_mesh(
        &mut self,
        handle: MeshInstanceHandle,
        mesh: MeshHandle,
    ) -> Option<()> {
        let instance = instance_slot_mut(&mut self.instances, handle)?
            .value
            .as_mut()?;
        instance.mesh = mesh;
        advance_revision(&mut self.instance_revision);
        Some(())
    }

    pub fn set_instance_material(
        &mut self,
        handle: MeshInstanceHandle,
        material: MaterialHandle,
    ) -> Option<()> {
        let instance = instance_slot_mut(&mut self.instances, handle)?
            .value
            .as_mut()?;
        instance.material = material;
        advance_revision(&mut self.instance_revision);
        Some(())
    }

    pub fn mesh_entries(&self) -> impl Iterator<Item = (MeshHandle, &Mesh, u64)> + '_ {
        self.meshes.iter().enumerate().filter_map(|(index, slot)| {
            slot.value
                .as_ref()
                .map(|mesh| (MeshHandle::new(index, slot.generation), mesh, slot.revision))
        })
    }

    pub fn mesh_slot_len(&self) -> usize {
        self.meshes.len()
    }

    pub fn mesh_revision_id(&self) -> u64 {
        self.mesh_revision
    }

    pub fn texture_entries(&self) -> impl Iterator<Item = (TextureHandle, &Texture, u64)> + '_ {
        self.textures
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.value.as_ref().map(|texture| {
                    (
                        TextureHandle::new(index, slot.generation),
                        texture,
                        slot.revision,
                    )
                })
            })
    }

    pub fn texture_slot_len(&self) -> usize {
        self.textures.len()
    }

    pub fn texture_revision_id(&self) -> u64 {
        self.texture_revision
    }

    pub fn material_entries(&self) -> impl Iterator<Item = (MaterialHandle, &Material, u64)> + '_ {
        self.materials
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.value.as_ref().map(|material| {
                    (
                        MaterialHandle::new(index, slot.generation),
                        material,
                        slot.revision,
                    )
                })
            })
    }

    pub fn material_slot_len(&self) -> usize {
        self.materials.len()
    }

    pub fn material_revision_id(&self) -> u64 {
        self.material_revision
    }

    pub fn instance_entries(
        &self,
    ) -> impl Iterator<Item = (MeshInstanceHandle, &MeshInstance)> + '_ {
        self.instances
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.value
                    .as_ref()
                    .map(|instance| (MeshInstanceHandle::new(index, slot.generation), instance))
            })
    }

    pub fn instance_slot_len(&self) -> usize {
        self.instances.len()
    }

    pub fn instance_count(&self) -> usize {
        self.instance_entries().count()
    }

    pub fn instance_revision_id(&self) -> u64 {
        self.instance_revision
    }

    pub fn camera(&self) -> Camera {
        self.camera
    }

    pub fn set_camera(&mut self, camera: impl Into<Camera>) {
        self.camera = camera.into();
    }

    pub fn clear_color(&self) -> Color {
        self.clear_color
    }

    pub fn set_clear_color(&mut self, clear_color: Color) {
        self.clear_color = clear_color;
    }

    pub fn depth(&self) -> RenderDepthDesc {
        self.depth
    }

    pub fn set_depth(&mut self, depth: RenderDepthDesc) {
        self.depth = depth;
    }

    pub fn lighting(&self) -> RenderLighting {
        self.lighting
    }

    pub fn set_lighting(&mut self, lighting: RenderLighting) {
        self.lighting = lighting;
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio.max(0.0001);
    }

    pub fn frustum_culling(&self) -> bool {
        self.frustum_culling
    }

    pub fn set_frustum_culling(&mut self, enabled: bool) {
        self.frustum_culling = enabled;
    }
}

fn insert_slot<T>(
    slots: &mut Vec<SceneSlot<T>>,
    free_list: &mut Vec<usize>,
    value: T,
    revision: &mut u64,
) -> (usize, u32) {
    let revision = advance_revision(revision);

    if let Some(index) = free_list.pop() {
        let slot = &mut slots[index];
        slot.value = Some(value);
        slot.revision = revision;
        (index, slot.handle_generation())
    } else {
        let index = slots.len();
        slots.push(SceneSlot::occupied(value, revision));
        (index, 0)
    }
}

fn remove_slot<T>(
    slots: &mut [SceneSlot<T>],
    free_list: &mut Vec<usize>,
    index: usize,
    generation: u32,
    revision: &mut u64,
) -> Option<T> {
    let slot = slots.get_mut(index)?;
    if slot.generation != generation || !slot.is_occupied() {
        return None;
    }

    let value = slot.value.take();
    slot.generation = next_generation(slot.generation);
    slot.revision = advance_revision(revision);
    free_list.push(index);
    value
}

fn mesh_slot(slots: &[SceneSlot<Mesh>], handle: MeshHandle) -> Option<&SceneSlot<Mesh>> {
    matching_slot(slots, handle.index(), handle.generation())
}

fn mesh_slot_mut(
    slots: &mut [SceneSlot<Mesh>],
    handle: MeshHandle,
) -> Option<&mut SceneSlot<Mesh>> {
    matching_slot_mut(slots, handle.index(), handle.generation())
}

fn texture_slot(
    slots: &[SceneSlot<Texture>],
    handle: TextureHandle,
) -> Option<&SceneSlot<Texture>> {
    matching_slot(slots, handle.index(), handle.generation())
}

fn texture_slot_mut(
    slots: &mut [SceneSlot<Texture>],
    handle: TextureHandle,
) -> Option<&mut SceneSlot<Texture>> {
    matching_slot_mut(slots, handle.index(), handle.generation())
}

fn material_slot(
    slots: &[SceneSlot<Material>],
    handle: MaterialHandle,
) -> Option<&SceneSlot<Material>> {
    matching_slot(slots, handle.index(), handle.generation())
}

fn material_slot_mut(
    slots: &mut [SceneSlot<Material>],
    handle: MaterialHandle,
) -> Option<&mut SceneSlot<Material>> {
    matching_slot_mut(slots, handle.index(), handle.generation())
}

fn instance_slot(
    slots: &[SceneSlot<MeshInstance>],
    handle: MeshInstanceHandle,
) -> Option<&SceneSlot<MeshInstance>> {
    matching_slot(slots, handle.index(), handle.generation())
}

fn instance_slot_mut(
    slots: &mut [SceneSlot<MeshInstance>],
    handle: MeshInstanceHandle,
) -> Option<&mut SceneSlot<MeshInstance>> {
    matching_slot_mut(slots, handle.index(), handle.generation())
}

fn matching_slot<T>(
    slots: &[SceneSlot<T>],
    index: usize,
    generation: u32,
) -> Option<&SceneSlot<T>> {
    let slot = slots.get(index)?;
    (slot.generation == generation && slot.value.is_some()).then_some(slot)
}

fn matching_slot_mut<T>(
    slots: &mut [SceneSlot<T>],
    index: usize,
    generation: u32,
) -> Option<&mut SceneSlot<T>> {
    let slot = slots.get_mut(index)?;
    (slot.generation == generation && slot.value.is_some()).then_some(slot)
}

fn next_generation(generation: u32) -> u32 {
    generation.wrapping_add(1)
}

fn advance_revision(revision: &mut u64) -> u64 {
    *revision = revision.wrapping_add(1).max(1);
    *revision
}

fn sampled_node_transforms(samples: &[GltfAnimationSample]) -> Vec<(usize, SampledNodeTransform)> {
    let mut nodes = Vec::<(usize, SampledNodeTransform)>::new();

    for sample in samples {
        let slot = if let Some(index) = nodes
            .iter()
            .position(|(node_index, _)| *node_index == sample.target_node)
        {
            &mut nodes[index].1
        } else {
            nodes.push((
                sample.target_node,
                SampledNodeTransform {
                    translation: None,
                    rotation: None,
                    scale: None,
                },
            ));
            &mut nodes.last_mut().expect("sampled node was just pushed").1
        };

        match (sample.path, &sample.value) {
            (GltfAnimationPath::Translation, GltfAnimationValue::Translation(value)) => {
                slot.translation = Some(*value);
            }
            (GltfAnimationPath::Rotation, GltfAnimationValue::Rotation(value)) => {
                slot.rotation = Some(*value);
            }
            (GltfAnimationPath::Scale, GltfAnimationValue::Scale(value)) => {
                slot.scale = Some(*value);
            }
            _ => {}
        }
    }

    nodes
}

fn sampled_node_matrices(sampled_nodes: &[(usize, SampledNodeTransform)]) -> Vec<(usize, Mat4)> {
    sampled_nodes
        .iter()
        .map(|(node_index, sampled)| {
            let translation = sampled.translation.unwrap_or([0.0, 0.0, 0.0]);
            let rotation = sampled.rotation.unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let scale = sampled.scale.unwrap_or([1.0, 1.0, 1.0]);
            (
                *node_index,
                Mat4::translation(translation)
                    * Mat4::rotation_quaternion(rotation)
                    * Mat4::scale(scale),
            )
        })
        .collect()
}

fn sampled_node_weights(samples: &[GltfAnimationSample]) -> Vec<(usize, Vec<f32>)> {
    let mut nodes = Vec::<(usize, Vec<f32>)>::new();

    for sample in samples {
        let (GltfAnimationPath::Weights, GltfAnimationValue::Weights(weights)) =
            (sample.path, &sample.value)
        else {
            continue;
        };

        if let Some((_, stored)) = nodes
            .iter_mut()
            .find(|(node_index, _)| *node_index == sample.target_node)
        {
            *stored = weights.clone();
        } else {
            nodes.push((sample.target_node, weights.clone()));
        }
    }

    nodes
}

fn load_gltf_texture(
    scene: &mut RenderScene,
    path: Option<&str>,
    image: Option<&GltfImageData>,
    texture_resolver: &mut impl FnMut(&str) -> Option<Texture>,
) -> Result<Option<TextureHandle>, GltfSceneLoadError> {
    if let Some(path) = path {
        let texture =
            texture_resolver(path).ok_or_else(|| GltfSceneLoadError::TextureNotFound {
                path: path.to_owned(),
            })?;
        return Ok(Some(scene.add_texture(texture)));
    }

    if let Some(image) = image {
        let texture = Texture::from_image_bytes(&image.label, &image.bytes)?;
        return Ok(Some(scene.add_texture(texture)));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BlendMode, DirectionalLight, DirectionalShadow, OrthographicCamera, RenderLighting,
        TextureSize,
    };

    #[test]
    fn mesh_handles_are_invalidated_when_slots_are_reused() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let first = scene.add_mesh(Mesh::colored_triangle());

        assert!(scene.mesh(first).is_some());
        assert!(scene.remove_mesh(first).is_some());
        assert!(scene.mesh(first).is_none());

        let reused = scene.add_mesh(Mesh::colored_triangle());

        assert_eq!(reused.index(), first.index());
        assert_ne!(reused.generation(), first.generation());
        assert!(scene.mesh(first).is_none());
        assert!(scene.mesh(reused).is_some());
    }

    #[test]
    fn stale_handles_cannot_mutate_reused_slots() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let material = scene.add_material(Material::new([1.0, 0.0, 0.0, 1.0]));

        assert!(scene.remove_material(material).is_some());
        let reused = scene.add_material(Material::new([0.0, 1.0, 0.0, 1.0]));

        assert_eq!(reused.index(), material.index());
        assert_ne!(reused.generation(), material.generation());
        assert!(scene.replace_material(material, Material::WHITE).is_none());
        assert!(scene.remove_material(material).is_none());
        assert_eq!(
            scene.material(reused).map(|material| material.tint),
            Some([0.0, 1.0, 0.0, 1.0])
        );
    }

    #[test]
    fn default_material_cannot_be_removed() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let default_material = scene.default_material();

        assert!(scene.remove_material(default_material).is_none());
        assert!(scene.material(default_material).is_some());
    }

    #[test]
    fn instance_revision_tracks_resource_binding_changes() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let first_mesh = scene.add_mesh(Mesh::colored_triangle());
        let second_mesh = scene.add_mesh(Mesh::colored_triangle());
        let material = scene.add_material(Material::new([0.0, 1.0, 0.0, 1.0]));
        let instance = scene.add_instance(first_mesh, Transform::IDENTITY);
        let after_add = scene.instance_revision_id();

        let moved = Transform::new([1.0, 2.0, 0.0], 0.25, [1.0, 1.0, 1.0]);
        assert_eq!(scene.set_instance_transform(instance, moved), Some(()));
        assert_eq!(scene.instance_revision_id(), after_add);
        assert_eq!(
            scene.instance(instance).map(|instance| instance.transform),
            Some(moved)
        );

        assert_eq!(scene.set_instance_mesh(instance, second_mesh), Some(()));
        let after_mesh_change = scene.instance_revision_id();
        assert!(after_mesh_change > after_add);

        assert_eq!(scene.set_instance_material(instance, material), Some(()));
        assert!(scene.instance_revision_id() > after_mesh_change);
    }

    #[test]
    fn matrix_instances_preserve_explicit_model_matrix() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let mesh = scene.add_mesh(Mesh::colored_triangle());
        let model_matrix = Mat4::translation([2.0, 3.0, 4.0]) * Mat4::scale([2.0, 1.0, 1.0]);
        let instance =
            scene.add_instance_with_material_matrix(mesh, scene.default_material(), model_matrix);

        let stored = scene.instance(instance).unwrap();
        assert_eq!(stored.model_matrix_override(), Some(model_matrix));
        assert_eq!(stored.model_matrix(), model_matrix);
        assert_eq!(stored.sort_position(), [2.0, 3.0, 4.0]);

        let replacement = Mat4::translation([-1.0, 0.5, 6.0]) * Mat4::scale([1.0, 3.0, 1.0]);
        scene
            .set_instance_model_matrix(instance, replacement)
            .unwrap();
        let stored = scene.instance(instance).unwrap();
        assert_eq!(stored.model_matrix_override(), Some(replacement));
        assert_eq!(stored.sort_position(), [-1.0, 0.5, 6.0]);

        scene
            .set_instance_transform(instance, Transform::IDENTITY)
            .unwrap();
        let stored = scene.instance(instance).unwrap();
        assert_eq!(stored.model_matrix_override(), None);
        assert_eq!(stored.model_matrix(), Transform::IDENTITY.to_matrix());
    }

    #[test]
    fn instance_visibility_and_sort_order_are_render_state_only() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let mesh = scene.add_mesh(Mesh::colored_triangle());
        let instance = scene.add_instance(mesh, Transform::IDENTITY);
        let after_add = scene.instance_revision_id();

        assert_eq!(scene.set_instance_visible(instance, false), Some(()));
        assert_eq!(scene.set_instance_sort_order(instance, 42), Some(()));

        let instance_state = scene.instance(instance).copied().unwrap();
        assert!(!instance_state.visible);
        assert_eq!(instance_state.sort_order, 42);
        assert_eq!(scene.instance_revision_id(), after_add);
    }

    #[test]
    fn texture_handles_are_generational() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let first = scene.add_texture(Texture::white_1x1());

        assert!(scene.remove_texture(first).is_some());
        let reused = scene.add_texture(Texture::solid_rgba(
            TextureSize::new(1, 1),
            [8, 16, 32, 255],
        ));

        assert_eq!(reused.index(), first.index());
        assert_ne!(reused.generation(), first.generation());
        assert!(scene.texture(first).is_none());
        assert!(scene.texture(reused).is_some());
    }

    #[test]
    fn depth_state_is_captured_by_render_queue() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        scene.set_depth(RenderDepthDesc::disabled());

        let queue = crate::RenderQueue::from_scene(&scene);

        assert_eq!(scene.depth(), RenderDepthDesc::DISABLED);
        assert_eq!(queue.pass().depth, RenderDepthDesc::DISABLED);
    }

    #[test]
    fn lighting_state_is_captured_by_render_queue() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let lighting = RenderLighting::new(
            [0.2, 0.3, 0.4],
            0.5,
            DirectionalLight::new([1.0, -2.0, 0.5], [0.8, 0.7, 0.6], 1.25),
        )
        .with_directional_shadow(DirectionalShadow::enabled(
            1024, 8.0, -10.0, 12.0, 0.5, 0.001,
        ));

        scene.set_lighting(lighting);
        let queue = crate::RenderQueue::from_scene(&scene);

        assert_eq!(scene.lighting(), lighting);
        assert_eq!(queue.pass().lighting, lighting);
    }

    #[test]
    fn obj_mtl_import_adds_meshes_materials_and_instances() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let imported = scene
            .add_obj_mtl_instances(
                "\
v 0 0 0
v 1 0 0
v 0 1 0
v 1 1 0
usemtl red
f 1 2 3
usemtl glass
f 2 4 3
",
                "\
newmtl red
Kd 1 0.1 0.1
Pr 0.3
newmtl glass
Kd 0.4 0.6 1
d 0.5
Pm 0.2
",
                Transform::IDENTITY,
            )
            .unwrap();

        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].material_name.as_deref(), Some("red"));
        assert_eq!(scene.mesh(imported[0].mesh).unwrap().vertex_count(), 3);
        assert_eq!(
            scene.material(imported[0].material).unwrap().tint,
            [1.0, 0.1, 0.1, 1.0]
        );
        assert_eq!(imported[1].material_name.as_deref(), Some("glass"));
        assert_eq!(
            scene.material(imported[1].material).unwrap().blend_mode,
            BlendMode::AlphaBlend
        );
        assert_eq!(
            scene.instance(imported[1].instance).unwrap().material,
            imported[1].material
        );
    }

    #[test]
    fn obj_mtl_import_can_resolve_base_color_textures() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let imported = scene
            .add_obj_mtl_instances_with_textures(
                "\
v 0 0 0
v 1 0 0
v 0 1 0
usemtl textured
f 1 2 3
",
                "\
newmtl textured
Kd 1 1 1
map_Kd albedo.png
",
                Transform::IDENTITY,
                |path| {
                    assert_eq!(path, "albedo.png");
                    Some(Texture::solid_rgba(
                        TextureSize::new(1, 1),
                        [32, 64, 128, 255],
                    ))
                },
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .unwrap();

        assert_eq!(
            scene.texture(texture).map(Texture::rgba8_data),
            Some([32, 64, 128, 255].as_slice())
        );
    }

    #[test]
    fn gltf_import_adds_mesh_material_texture_and_instance() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let transform = Transform::new([0.25, -0.5, 0.0], 0.0, [2.0, 2.0, 1.0]);
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                r#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "tri.bin", "byteLength": 42 }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
  ],
  "images": [
    { "uri": "albedo.bmp" },
    { "uri": "surface.bmp" },
    { "uri": "normal.bmp" },
    { "uri": "emissive.bmp" },
    { "uri": "occlusion.bmp" }
  ],
  "textures": [
    { "source": 0 },
    { "source": 1 },
    { "source": 2 },
    { "source": 3 },
    { "source": 4 }
  ],
  "materials": [{
    "normalTexture": { "index": 2, "scale": 0.25 },
    "emissiveTexture": { "index": 3 },
    "emissiveFactor": [0.2, 0.3, 0.4],
    "occlusionTexture": { "index": 4, "strength": 0.6 },
    "alphaMode": "BLEND",
    "extensions": {
      "KHR_materials_clearcoat": {
        "clearcoatFactor": 0.75,
        "clearcoatRoughnessFactor": 0.2,
        "clearcoatTexture": { "index": 0 },
        "clearcoatRoughnessTexture": { "index": 1 }
      },
      "KHR_materials_sheen": {
        "sheenColorFactor": [0.1, 0.2, 0.3],
        "sheenRoughnessFactor": 0.4,
        "sheenColorTexture": { "index": 3 },
        "sheenRoughnessTexture": { "index": 4 }
      },
      "KHR_materials_transmission": {
        "transmissionFactor": 0.5,
        "transmissionTexture": { "index": 0 }
      },
      "KHR_materials_specular": {
        "specularFactor": 0.7,
        "specularColorFactor": [0.8, 0.9, 1.0],
        "specularTexture": { "index": 4 },
        "specularColorTexture": { "index": 3 }
      },
      "KHR_materials_anisotropy": {
        "anisotropyStrength": 0.55,
        "anisotropyRotation": 0.25,
        "anisotropyTexture": { "index": 1 }
      },
      "KHR_materials_iridescence": {
        "iridescenceFactor": 0.4,
        "iridescenceIor": 1.45,
        "iridescenceThicknessMinimum": 120.0,
        "iridescenceThicknessMaximum": 380.0,
        "iridescenceTexture": { "index": 2 },
        "iridescenceThicknessTexture": { "index": 1 }
      },
      "KHR_materials_volume": {
        "thicknessFactor": 0.35,
        "attenuationColor": [0.7, 0.8, 0.9],
        "attenuationDistance": 2.5,
        "thicknessTexture": { "index": 4 }
      },
      "KHR_materials_dispersion": {
        "dispersion": 0.12
      }
    },
    "pbrMetallicRoughness": {
      "baseColorFactor": [0.2, 0.4, 0.8, 0.5],
      "roughnessFactor": 0.7,
      "metallicFactor": 0.1,
      "baseColorTexture": { "index": 0 },
      "metallicRoughnessTexture": { "index": 1 }
    }
  }],
  "meshes": [{
    "primitives": [{
      "attributes": { "POSITION": 0 },
      "indices": 1,
      "material": 0
    }]
  }]
}"#,
                transform,
                |path| (path == "tri.bin").then(triangle_gltf_buffer),
                |path| match path {
                    "albedo.bmp" => Some(Texture::solid_rgba(
                        TextureSize::new(1, 1),
                        [10, 20, 30, 255],
                    )),
                    "surface.bmp" => Some(Texture::solid_rgba(
                        TextureSize::new(1, 1),
                        [255, 64, 128, 255],
                    )),
                    "normal.bmp" => Some(Texture::solid_rgba(
                        TextureSize::new(1, 1),
                        [128, 128, 255, 255],
                    )),
                    "emissive.bmp" => {
                        Some(Texture::solid_rgba(TextureSize::new(1, 1), [4, 8, 12, 255]))
                    }
                    "occlusion.bmp" => Some(Texture::solid_rgba(
                        TextureSize::new(1, 1),
                        [160, 0, 0, 255],
                    )),
                    _ => None,
                },
            )
            .unwrap();

        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].node_index, None);
        assert_eq!(imported[0].mesh_index, 0);
        assert_eq!(imported[0].primitive_index, 0);
        assert_eq!(imported[0].material_index, Some(0));
        assert_eq!(scene.mesh(imported[0].mesh).unwrap().vertex_count(), 3);
        assert_eq!(scene.mesh(imported[0].mesh).unwrap().indices(), &[0, 1, 2]);
        let material = scene.material(imported[0].material).unwrap();
        assert_eq!(material.tint, [0.2, 0.4, 0.8, 0.5]);
        assert_eq!(material.blend_mode, BlendMode::AlphaBlend);
        assert_eq!(material.roughness, 0.7);
        assert_eq!(material.metallic, 0.1);
        assert_eq!(material.normal_scale, 0.25);
        assert_eq!(material.emissive, [0.2, 0.3, 0.4]);
        assert_eq!(material.occlusion_strength, 0.6);
        assert_eq!(material.clearcoat, 0.75);
        assert_eq!(material.clearcoat_roughness, 0.2);
        assert_eq!(material.sheen_color, [0.1, 0.2, 0.3]);
        assert_eq!(material.sheen_roughness, 0.4);
        assert_eq!(material.transmission, 0.5);
        assert_eq!(material.specular_factor, 0.7);
        assert_eq!(material.specular_color, [0.8, 0.9, 1.0]);
        assert_eq!(material.anisotropy_strength, 0.55);
        assert_eq!(material.anisotropy_rotation, 0.25);
        assert_eq!(material.iridescence_factor, 0.4);
        assert_eq!(material.iridescence_ior, 1.45);
        assert_eq!(material.iridescence_thickness_min, 120.0);
        assert_eq!(material.iridescence_thickness_max, 380.0);
        assert_eq!(material.thickness_factor, 0.35);
        assert_eq!(material.attenuation_color, [0.7, 0.8, 0.9]);
        assert_eq!(material.attenuation_distance, 2.5);
        assert_eq!(material.dispersion, 0.12);
        let texture = material.base_color_texture.unwrap();
        assert_eq!(
            scene.texture(texture).map(Texture::rgba8_data),
            Some([10, 20, 30, 255].as_slice())
        );
        let surface_texture = material.metallic_roughness_texture.unwrap();
        assert_eq!(
            scene.texture(surface_texture).map(Texture::rgba8_data),
            Some([255, 64, 128, 255].as_slice())
        );
        let normal_texture = material.normal_texture.unwrap();
        assert_eq!(
            scene.texture(normal_texture).map(Texture::rgba8_data),
            Some([128, 128, 255, 255].as_slice())
        );
        let emissive_texture = material.emissive_texture.unwrap();
        assert_eq!(
            scene.texture(emissive_texture).map(Texture::rgba8_data),
            Some([4, 8, 12, 255].as_slice())
        );
        let occlusion_texture = material.occlusion_texture.unwrap();
        assert_eq!(
            scene.texture(occlusion_texture).map(Texture::rgba8_data),
            Some([160, 0, 0, 255].as_slice())
        );
        let clearcoat_texture = material.clearcoat_texture.unwrap();
        assert_eq!(
            scene.texture(clearcoat_texture).map(Texture::rgba8_data),
            Some([10, 20, 30, 255].as_slice())
        );
        let clearcoat_roughness_texture = material.clearcoat_roughness_texture.unwrap();
        assert_eq!(
            scene
                .texture(clearcoat_roughness_texture)
                .map(Texture::rgba8_data),
            Some([255, 64, 128, 255].as_slice())
        );
        let sheen_color_texture = material.sheen_color_texture.unwrap();
        assert_eq!(
            scene.texture(sheen_color_texture).map(Texture::rgba8_data),
            Some([4, 8, 12, 255].as_slice())
        );
        let sheen_roughness_texture = material.sheen_roughness_texture.unwrap();
        assert_eq!(
            scene
                .texture(sheen_roughness_texture)
                .map(Texture::rgba8_data),
            Some([160, 0, 0, 255].as_slice())
        );
        let transmission_texture = material.transmission_texture.unwrap();
        assert_eq!(
            scene.texture(transmission_texture).map(Texture::rgba8_data),
            Some([10, 20, 30, 255].as_slice())
        );
        let specular_texture = material.specular_texture.unwrap();
        assert_eq!(
            scene.texture(specular_texture).map(Texture::rgba8_data),
            Some([160, 0, 0, 255].as_slice())
        );
        let specular_color_texture = material.specular_color_texture.unwrap();
        assert_eq!(
            scene
                .texture(specular_color_texture)
                .map(Texture::rgba8_data),
            Some([4, 8, 12, 255].as_slice())
        );
        let anisotropy_texture = material.anisotropy_texture.unwrap();
        assert_eq!(
            scene.texture(anisotropy_texture).map(Texture::rgba8_data),
            Some([255, 64, 128, 255].as_slice())
        );
        let iridescence_texture = material.iridescence_texture.unwrap();
        assert_eq!(
            scene.texture(iridescence_texture).map(Texture::rgba8_data),
            Some([128, 128, 255, 255].as_slice())
        );
        let iridescence_thickness_texture = material.iridescence_thickness_texture.unwrap();
        assert_eq!(
            scene
                .texture(iridescence_thickness_texture)
                .map(Texture::rgba8_data),
            Some([255, 64, 128, 255].as_slice())
        );
        let thickness_texture = material.thickness_texture.unwrap();
        assert_eq!(
            scene.texture(thickness_texture).map(Texture::rgba8_data),
            Some([160, 0, 0, 255].as_slice())
        );
        assert_eq!(
            scene
                .instance(imported[0].instance)
                .map(|instance| instance.transform),
            Some(transform)
        );
    }

    #[test]
    fn gltf_import_resolves_ext_texture_webp_external_texture() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                r#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "tri.bin", "byteLength": 42 }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
  ],
  "images": [
    { "uri": "fallback.png" },
    { "uri": "albedo.webp" }
  ],
  "textures": [{
    "source": 0,
    "extensions": { "EXT_texture_webp": { "source": 1 } }
  }],
  "materials": [{
    "pbrMetallicRoughness": {
      "baseColorTexture": { "index": 0 }
    }
  }],
  "meshes": [{
    "primitives": [{
      "attributes": { "POSITION": 0 },
      "indices": 1,
      "material": 0
    }]
  }]
}"#,
                Transform::IDENTITY,
                |path| (path == "tri.bin").then(triangle_gltf_buffer),
                |path| {
                    assert_eq!(path, "albedo.webp");
                    Some(Texture::solid_rgba(
                        TextureSize::new(1, 1),
                        [7, 11, 13, 255],
                    ))
                },
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .unwrap();

        assert_eq!(
            scene.texture(texture).map(Texture::rgba8_data),
            Some([7, 11, 13, 255].as_slice())
        );
    }

    #[test]
    fn gltf_import_resolves_khr_texture_basisu_external_texture() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                r#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "tri.bin", "byteLength": 42 }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
  ],
  "images": [
    { "uri": "fallback.png" },
    { "uri": "albedo.ktx2" }
  ],
  "textures": [{
    "source": 0,
    "extensions": { "KHR_texture_basisu": { "source": 1 } }
  }],
  "materials": [{
    "pbrMetallicRoughness": {
      "baseColorTexture": { "index": 0 }
    }
  }],
  "meshes": [{
    "primitives": [{
      "attributes": { "POSITION": 0 },
      "indices": 1,
      "material": 0
    }]
  }]
}"#,
                Transform::IDENTITY,
                |path| (path == "tri.bin").then(triangle_gltf_buffer),
                |path| {
                    assert_eq!(path, "albedo.ktx2");
                    Some(Texture::solid_rgba(
                        TextureSize::new(1, 1),
                        [17, 19, 23, 255],
                    ))
                },
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .unwrap();

        assert_eq!(
            scene.texture(texture).map(Texture::rgba8_data),
            Some([17, 19, 23, 255].as_slice())
        );
    }

    #[test]
    fn gltf_import_applies_node_model_matrix_to_scene_instance() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let transform = Transform::new_3d([10.0, 0.0, 0.0], [0.0; 3], [1.0, 1.0, 1.0]);
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                r#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "tri.bin", "byteLength": 42 }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
  ],
  "nodes": [{ "mesh": 0, "translation": [1.0, 2.0, 3.0] }],
  "scenes": [{ "nodes": [0] }],
  "scene": 0,
  "meshes": [{
    "primitives": [{
      "attributes": { "POSITION": 0 },
      "indices": 1
    }]
  }]
}"#,
                transform,
                |path| (path == "tri.bin").then(triangle_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let expected = transform.to_matrix() * Mat4::translation([1.0, 2.0, 3.0]);
        assert_eq!(imported[0].node_index, Some(0));
        let instance = scene.instance(imported[0].instance).unwrap();
        assert_eq!(instance.model_matrix_override(), Some(expected));
        assert_eq!(instance.sort_position(), [11.0, 2.0, 3.0]);
    }

    #[test]
    fn gltf_import_applies_punctual_lights_to_scene_lighting() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        scene
            .add_gltf_instances_with_buffers_and_textures(
                r#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "tri.bin", "byteLength": 42 }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
  ],
  "extensions": {
    "KHR_lights_punctual": {
      "lights": [
        {
          "type": "point",
          "color": [0.8, 0.6, 0.4],
          "intensity": 4.0,
          "range": 9.0
        }
      ]
    }
  },
  "nodes": [
    { "mesh": 0 },
    {
      "translation": [1.0, 2.0, 3.0],
      "extensions": { "KHR_lights_punctual": { "light": 0 } }
    }
  ],
  "scenes": [{ "nodes": [0, 1] }],
  "scene": 0,
  "meshes": [{
    "primitives": [{
      "attributes": { "POSITION": 0 },
      "indices": 1
    }]
  }]
}"#,
                Transform::new_3d([10.0, 0.0, 0.0], [0.0; 3], [1.0, 1.0, 1.0]),
                |path| (path == "tri.bin").then(triangle_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let lighting = scene.lighting();
        assert_eq!(lighting.ambient_intensity, 0.0);
        assert_eq!(lighting.directional.intensity, 0.0);
        assert_eq!(lighting.point_lights().len(), 1);
        assert_eq!(lighting.point_lights()[0].position, [11.0, 2.0, 3.0]);
        assert_eq!(lighting.point_lights()[0].color, [0.8, 0.6, 0.4]);
        assert_eq!(lighting.point_lights()[0].intensity, 4.0);
        assert_eq!(lighting.point_lights()[0].range, 9.0);
    }

    #[test]
    fn gltf_animation_samples_can_drive_imported_node_instances() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let asset = GltfAsset::from_gltf_str_with_buffers(animated_translation_gltf(), |uri| {
            (uri == "animated.bin").then(animated_translation_buffer)
        })
        .unwrap();
        let samples = asset.animations[0].sample(1.0);
        let imported = scene
            .add_gltf_asset_instances_with_textures(
                asset,
                Transform::new_3d([10.0, 0.0, 0.0], [0.0; 3], [1.0, 1.0, 1.0]),
                |_| None,
            )
            .unwrap();

        let updated = scene.apply_gltf_animation_samples_to_imported_parts(
            &imported,
            Transform::new_3d([10.0, 0.0, 0.0], [0.0; 3], [1.0, 1.0, 1.0]),
            &samples,
        );

        assert_eq!(updated, 1);
        assert_eq!(imported[0].node_index, Some(0));
        assert_eq!(
            scene
                .instance(imported[0].instance)
                .and_then(|instance| instance.model_matrix_override()),
            Some(Mat4::translation([12.0, 0.0, 0.0]))
        );
    }

    #[test]
    fn gltf_animation_weight_samples_can_drive_imported_morph_meshes() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let asset = GltfAsset::from_gltf_str_with_buffers(animated_morph_weights_gltf(), |uri| {
            (uri == "morph_anim.bin").then(animated_morph_weights_buffer)
        })
        .unwrap();
        let samples = asset.animations[0].sample(1.0);
        let imported = scene
            .add_gltf_asset_instances_with_textures(asset, Transform::IDENTITY, |_| None)
            .unwrap();

        assert_eq!(
            scene.mesh(imported[0].mesh).unwrap().vertices()[0].position,
            [0.0, 0.0, 0.0]
        );

        let updated = scene.apply_gltf_animation_samples_to_imported_parts(
            &imported,
            Transform::IDENTITY,
            &samples,
        );

        assert_eq!(updated, 1);
        let mesh = scene.mesh(imported[0].mesh).unwrap();
        assert_eq!(mesh.vertices()[0].position, [0.0, 0.0, 0.5]);
        assert_eq!(mesh.vertices()[1].position, [1.0, 0.0, 0.5]);
    }

    #[test]
    fn gltf_animation_joint_samples_can_drive_imported_skinned_meshes() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let asset = GltfAsset::from_gltf_str_with_buffers(animated_skin_gltf(), |uri| {
            (uri == "skin_anim.bin").then(animated_skin_buffer)
        })
        .unwrap();
        let samples = asset.animations[0].sample(1.0);
        let imported = scene
            .add_gltf_asset_instances_with_textures(asset, Transform::IDENTITY, |_| None)
            .unwrap();

        assert_eq!(
            scene.mesh(imported[0].mesh).unwrap().vertices()[0].position,
            [0.0, 0.0, 1.0]
        );

        let updated = scene.apply_gltf_animation_samples_to_imported_parts(
            &imported,
            Transform::IDENTITY,
            &samples,
        );

        assert_eq!(updated, 1);
        let mesh = scene.mesh(imported[0].mesh).unwrap();
        assert_eq!(mesh.vertices()[0].position, [0.0, 0.0, 2.0]);
        assert_eq!(mesh.vertices()[1].position, [1.0, 0.0, 2.0]);
    }

    #[test]
    fn gltf_import_decodes_embedded_base_color_texture() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let image_len = bmp_32_top_down_1x1().len();
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "asset.bin", "byteLength": {} }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 42, "byteLength": {image_len} }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "bufferView": 2, "mimeType": "image/bmp", "name": "embedded_albedo" }}],
  "textures": [{{ "source": 0 }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    42 + image_len
                ),
                Transform::IDENTITY,
                |path| (path == "asset.bin").then(embedded_texture_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .unwrap();
        assert_eq!(
            scene.texture(texture).map(Texture::rgba8_data),
            Some([10, 20, 30, 40].as_slice())
        );
    }

    #[test]
    fn gltf_import_decodes_embedded_texture_without_mime_label() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let image_len = bmp_32_top_down_1x1().len();
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "asset.bin", "byteLength": {} }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 42, "byteLength": {image_len} }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "bufferView": 2 }}],
  "textures": [{{ "source": 0 }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    42 + image_len
                ),
                Transform::IDENTITY,
                |path| (path == "asset.bin").then(embedded_texture_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .unwrap();
        assert_eq!(
            scene.texture(texture).map(Texture::rgba8_data),
            Some([10, 20, 30, 40].as_slice())
        );
    }

    #[test]
    fn gltf_import_decodes_embedded_tga_texture_without_mime_label() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let image_len = tga_32_top_left_1x1().len();
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "asset.bin", "byteLength": {} }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 42, "byteLength": {image_len} }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "bufferView": 2 }}],
  "textures": [{{ "source": 0 }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    42 + image_len
                ),
                Transform::IDENTITY,
                |path| (path == "asset.bin").then(embedded_tga_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .and_then(|texture| scene.texture(texture))
            .unwrap();
        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn gltf_import_decodes_embedded_color_mapped_tga_without_mime_label() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let image_len = tga_color_mapped_top_left_1x1().len();
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "asset.bin", "byteLength": {} }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 42, "byteLength": {image_len} }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "bufferView": 2 }}],
  "textures": [{{ "source": 0 }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    42 + image_len
                ),
                Transform::IDENTITY,
                |path| (path == "asset.bin").then(embedded_color_mapped_tga_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .and_then(|texture| scene.texture(texture))
            .unwrap();
        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[255, 0, 0, 255]);
    }

    #[test]
    fn gltf_import_decodes_embedded_webp_texture_data_uri() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "mesh.bin", "byteLength": 42 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "uri": "data:image/webp;base64,{}" }}],
  "textures": [{{ "source": 0 }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    valid_webp_1x1_base64()
                ),
                Transform::IDENTITY,
                |path| (path == "mesh.bin").then(triangle_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .and_then(|texture| scene.texture(texture))
            .unwrap();
        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data().len(), 4);
    }

    #[test]
    fn gltf_import_decodes_embedded_tga_texture_buffer_view() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let image_len = tga_32_top_left_1x1().len();
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "asset.bin", "byteLength": {} }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 42, "byteLength": {image_len} }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "bufferView": 2, "mimeType": "image/tga", "name": "embedded_albedo" }}],
  "textures": [{{ "source": 0 }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    42 + image_len
                ),
                Transform::IDENTITY,
                |path| (path == "asset.bin").then(embedded_tga_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .and_then(|texture| scene.texture(texture))
            .unwrap();
        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn gltf_import_decodes_embedded_tga_texture_data_uri() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "mesh.bin", "byteLength": 42 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "uri": "data:image/x-tga;base64,{}" }}],
  "textures": [{{ "source": 0 }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    tga_32_top_left_1x1_base64()
                ),
                Transform::IDENTITY,
                |path| (path == "mesh.bin").then(triangle_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .and_then(|texture| scene.texture(texture))
            .unwrap();
        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn gltf_import_decodes_embedded_ktx2_texture_buffer_view() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let image_len = ktx2_rgba8_1x1().len();
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "asset.bin", "byteLength": {} }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 42, "byteLength": {image_len} }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "bufferView": 2, "mimeType": "image/ktx2", "name": "albedo" }}],
  "textures": [{{
    "extensions": {{ "KHR_texture_basisu": {{ "source": 0 }} }}
  }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    42 + image_len
                ),
                Transform::IDENTITY,
                |path| (path == "asset.bin").then(embedded_ktx2_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .and_then(|texture| scene.texture(texture))
            .unwrap();
        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn gltf_import_decodes_embedded_zlib_ktx2_texture_buffer_view() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let image_len = ktx2_rgba8_zlib_1x1().len();
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "asset.bin", "byteLength": {} }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 42, "byteLength": {image_len} }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "bufferView": 2, "mimeType": "image/ktx2", "name": "albedo" }}],
  "textures": [{{
    "extensions": {{ "KHR_texture_basisu": {{ "source": 0 }} }}
  }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    42 + image_len
                ),
                Transform::IDENTITY,
                |path| (path == "asset.bin").then(embedded_ktx2_zlib_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .and_then(|texture| scene.texture(texture))
            .unwrap();
        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn gltf_import_decodes_embedded_zlib_ktx2_texture_buffer_view_with_x_mime() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let image_len = ktx2_rgba8_zlib_1x1().len();
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "asset.bin", "byteLength": {} }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 42, "byteLength": {image_len} }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "bufferView": 2, "mimeType": "image/x-ktx2", "name": "albedo" }}],
  "textures": [{{
    "extensions": {{ "KHR_texture_basisu": {{ "source": 0 }} }}
  }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    42 + image_len
                ),
                Transform::IDENTITY,
                |path| (path == "asset.bin").then(embedded_ktx2_zlib_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .and_then(|texture| scene.texture(texture))
            .unwrap();
        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn gltf_import_decodes_embedded_zlib_ktx2_texture_data_uri() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "mesh.bin", "byteLength": 42 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{
    "uri": "data:image/ktx2;base64,{}",
    "name": "albedo"
  }}],
  "textures": [{{
    "extensions": {{ "KHR_texture_basisu": {{ "source": 0 }} }}
  }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    ktx2_rgba8_zlib_1x1_base64()
                ),
                Transform::IDENTITY,
                |path| (path == "mesh.bin").then(triangle_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .and_then(|texture| scene.texture(texture))
            .unwrap();
        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn gltf_import_decodes_embedded_zlib_ktx2_texture_data_uri_with_x_mime() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "mesh.bin", "byteLength": 42 }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{
    "uri": "data:image/x-ktx2;base64,{}",
    "name": "albedo"
  }}],
  "textures": [{{
    "extensions": {{ "KHR_texture_basisu": {{ "source": 0 }} }}
  }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    ktx2_rgba8_zlib_1x1_base64()
                ),
                Transform::IDENTITY,
                |path| (path == "mesh.bin").then(triangle_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .and_then(|texture| scene.texture(texture))
            .unwrap();
        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn gltf_import_decodes_embedded_bgra8_ktx2_texture_buffer_view() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let image_len = ktx2_bgra8_1x1().len();
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                &format!(
                    r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [{{ "uri": "asset.bin", "byteLength": {} }}],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
    {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
    {{ "buffer": 0, "byteOffset": 42, "byteLength": {image_len} }}
  ],
  "accessors": [
    {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
    {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
  ],
  "images": [{{ "bufferView": 2, "mimeType": "image/ktx2", "name": "albedo" }}],
  "textures": [{{
    "extensions": {{ "KHR_texture_basisu": {{ "source": 0 }} }}
  }}],
  "materials": [{{
    "pbrMetallicRoughness": {{
      "baseColorTexture": {{ "index": 0 }}
    }}
  }}],
  "meshes": [{{
    "primitives": [{{
      "attributes": {{ "POSITION": 0 }},
      "indices": 1,
      "material": 0
    }}]
  }}]
}}"#,
                    42 + image_len
                ),
                Transform::IDENTITY,
                |path| (path == "asset.bin").then(embedded_bgra8_ktx2_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let texture = scene
            .material(imported[0].material)
            .and_then(|material| material.base_color_texture)
            .and_then(|texture| scene.texture(texture))
            .unwrap();
        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn gltf_import_adds_morphed_mesh_to_scene() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let imported = scene
            .add_gltf_instances_with_buffers_and_textures(
                r#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "morph.bin", "byteLength": 78 }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 },
    { "buffer": 0, "byteOffset": 42, "byteLength": 36 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" },
    { "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC3" }
  ],
  "meshes": [{
    "weights": [0.25],
    "primitives": [{
      "attributes": { "POSITION": 0 },
      "indices": 1,
      "targets": [{ "POSITION": 2 }]
    }]
  }]
}"#,
                Transform::IDENTITY,
                |path| (path == "morph.bin").then(morphed_triangle_gltf_buffer),
                |_| None,
            )
            .unwrap();

        let mesh = scene.mesh(imported[0].mesh).unwrap();
        assert_eq!(mesh.vertices()[0].position, [0.0, 0.0, 0.25]);
        assert_eq!(mesh.vertices()[1].position, [1.0, 0.0, 0.25]);
    }

    #[test]
    fn glb_import_uses_binary_chunk_for_default_buffer() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let glb = triangle_glb();
        let imported = scene
            .add_glb_instances_with_buffers_and_textures(
                &glb,
                Transform::IDENTITY,
                |_| None,
                |_| None,
            )
            .unwrap();

        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].material_index, None);
        assert_eq!(scene.mesh(imported[0].mesh).unwrap().vertex_count(), 3);
        assert_eq!(scene.mesh(imported[0].mesh).unwrap().indices(), &[0, 1, 2]);
        assert_eq!(
            scene
                .instance(imported[0].instance)
                .map(|instance| instance.material),
            Some(scene.default_material())
        );
    }

    fn triangle_gltf_buffer() -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0u16, 1, 2] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn animated_translation_gltf() -> &'static str {
        r#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "animated.bin", "byteLength": 76 }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 },
    { "buffer": 0, "byteOffset": 44, "byteLength": 8 },
    { "buffer": 0, "byteOffset": 52, "byteLength": 24 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" },
    { "bufferView": 2, "componentType": 5126, "count": 2, "type": "SCALAR" },
    { "bufferView": 3, "componentType": 5126, "count": 2, "type": "VEC3" }
  ],
  "nodes": [{ "mesh": 0 }],
  "scenes": [{ "nodes": [0] }],
  "scene": 0,
  "animations": [{
    "samplers": [{ "input": 2, "output": 3 }],
    "channels": [{ "sampler": 0, "target": { "node": 0, "path": "translation" } }]
  }],
  "meshes": [{
    "primitives": [{
      "attributes": { "POSITION": 0 },
      "indices": 1
    }]
  }]
}"#
    }

    fn animated_morph_weights_gltf() -> &'static str {
        r#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "morph_anim.bin", "byteLength": 94 }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 },
    { "buffer": 0, "byteOffset": 42, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 78, "byteLength": 8 },
    { "buffer": 0, "byteOffset": 86, "byteLength": 8 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" },
    { "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 3, "componentType": 5126, "count": 2, "type": "SCALAR" },
    { "bufferView": 4, "componentType": 5126, "count": 2, "type": "SCALAR" }
  ],
  "nodes": [{ "mesh": 0 }],
  "scenes": [{ "nodes": [0] }],
  "scene": 0,
  "meshes": [{
    "weights": [0.0],
    "primitives": [{
      "attributes": { "POSITION": 0 },
      "indices": 1,
      "targets": [{ "POSITION": 2 }]
    }]
  }],
  "animations": [{
    "samplers": [{ "input": 3, "output": 4, "interpolation": "LINEAR" }],
    "channels": [{ "sampler": 0, "target": { "node": 0, "path": "weights" } }]
  }]
}"#
    }

    fn animated_skin_gltf() -> &'static str {
        r#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "uri": "skin_anim.bin", "byteLength": 200 }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 },
    { "buffer": 0, "byteOffset": 42, "byteLength": 12 },
    { "buffer": 0, "byteOffset": 54, "byteLength": 48 },
    { "buffer": 0, "byteOffset": 102, "byteLength": 64 },
    { "buffer": 0, "byteOffset": 168, "byteLength": 8 },
    { "buffer": 0, "byteOffset": 176, "byteLength": 24 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" },
    { "bufferView": 2, "componentType": 5121, "count": 3, "type": "VEC4" },
    { "bufferView": 3, "componentType": 5126, "count": 3, "type": "VEC4" },
    { "bufferView": 4, "componentType": 5126, "count": 1, "type": "MAT4" },
    { "bufferView": 5, "componentType": 5126, "count": 2, "type": "SCALAR" },
    { "bufferView": 6, "componentType": 5126, "count": 2, "type": "VEC3" }
  ],
  "nodes": [
    { "mesh": 0, "skin": 0 },
    { "translation": [0.0, 0.0, 1.0], "children": [2] },
    { "translation": [0.0, 0.0, 1.0] }
  ],
  "skins": [{ "joints": [2], "inverseBindMatrices": 4 }],
  "scenes": [{ "nodes": [0, 1] }],
  "scene": 0,
  "animations": [{
    "samplers": [{ "input": 5, "output": 6, "interpolation": "LINEAR" }],
    "channels": [{ "sampler": 0, "target": { "node": 1, "path": "translation" } }]
  }],
  "meshes": [{
    "primitives": [{
      "attributes": { "POSITION": 0, "JOINTS_0": 2, "WEIGHTS_0": 3 },
      "indices": 1
    }]
  }]
}"#
    }

    fn animated_translation_buffer() -> Vec<u8> {
        let mut bytes = triangle_gltf_buffer();
        bytes.extend_from_slice(&[0; 2]);
        for value in [0.0f32, 2.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 0.0, 4.0, 0.0, 0.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn animated_morph_weights_buffer() -> Vec<u8> {
        let mut bytes = morphed_triangle_gltf_buffer();
        for value in [0.0f32, 2.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn animated_skin_buffer() -> Vec<u8> {
        let mut bytes = skinned_triangle_gltf_buffer();
        bytes.extend_from_slice(&[0; 2]);
        for value in [0.0f32, 2.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 3.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn embedded_texture_gltf_buffer() -> Vec<u8> {
        let mut bytes = triangle_gltf_buffer();
        bytes.extend_from_slice(&bmp_32_top_down_1x1());
        bytes
    }

    fn embedded_tga_gltf_buffer() -> Vec<u8> {
        let mut bytes = triangle_gltf_buffer();
        bytes.extend_from_slice(&tga_32_top_left_1x1());
        bytes
    }

    fn embedded_color_mapped_tga_gltf_buffer() -> Vec<u8> {
        let mut bytes = triangle_gltf_buffer();
        bytes.extend_from_slice(&tga_color_mapped_top_left_1x1());
        bytes
    }

    fn embedded_ktx2_gltf_buffer() -> Vec<u8> {
        let mut bytes = triangle_gltf_buffer();
        bytes.extend_from_slice(&ktx2_rgba8_1x1());
        bytes
    }

    fn embedded_ktx2_zlib_gltf_buffer() -> Vec<u8> {
        let mut bytes = triangle_gltf_buffer();
        bytes.extend_from_slice(&ktx2_rgba8_zlib_1x1());
        bytes
    }

    fn embedded_bgra8_ktx2_gltf_buffer() -> Vec<u8> {
        let mut bytes = triangle_gltf_buffer();
        bytes.extend_from_slice(&ktx2_bgra8_1x1());
        bytes
    }

    fn valid_webp_1x1_base64() -> &'static str {
        "UklGRiIAAABXRUJQVlA4IBYAAAAwAQCdASoBAAEADsD+JaQAA3AAAAAA"
    }

    fn tga_32_top_left_1x1_base64() -> String {
        encode_base64(&tga_32_top_left_1x1())
    }

    fn ktx2_rgba8_zlib_1x1_base64() -> String {
        encode_base64(&ktx2_rgba8_zlib_1x1())
    }

    fn ktx2_rgba8_1x1() -> Vec<u8> {
        ktx2_rgba8_level(&[10, 20, 30, 40], 0)
    }

    fn ktx2_bgra8_1x1() -> Vec<u8> {
        ktx2_bgra8_level(&[30, 20, 10, 40])
    }

    fn ktx2_rgba8_zlib_1x1() -> Vec<u8> {
        let pixel = [10, 20, 30, 40];
        ktx2_rgba8_level(&zlib_store_block(&pixel), 3)
    }

    fn ktx2_rgba8_level(level: &[u8], supercompression: u32) -> Vec<u8> {
        const HEADER_LEN: usize = 80;
        const LEVEL_INDEX_LEN: usize = 24;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\xabKTX 20\xbb\r\n\x1a\n");
        bytes.extend_from_slice(&37u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&supercompression.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        debug_assert_eq!(bytes.len(), HEADER_LEN);
        bytes.extend_from_slice(&((HEADER_LEN + LEVEL_INDEX_LEN) as u64).to_le_bytes());
        bytes.extend_from_slice(&(level.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&4u64.to_le_bytes());
        bytes.extend_from_slice(level);
        bytes
    }

    fn ktx2_bgra8_level(level: &[u8]) -> Vec<u8> {
        const HEADER_LEN: usize = 80;
        const LEVEL_INDEX_LEN: usize = 24;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\xabKTX 20\xbb\r\n\x1a\n");
        bytes.extend_from_slice(&44u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        debug_assert_eq!(bytes.len(), HEADER_LEN);
        bytes.extend_from_slice(&((HEADER_LEN + LEVEL_INDEX_LEN) as u64).to_le_bytes());
        bytes.extend_from_slice(&(level.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&4u64.to_le_bytes());
        bytes.extend_from_slice(level);
        bytes
    }

    fn zlib_store_block(raw: &[u8]) -> Vec<u8> {
        let mut zlib = Vec::new();
        zlib.extend_from_slice(&[0x78, 0x01, 0x01]);
        zlib.extend_from_slice(&(raw.len() as u16).to_le_bytes());
        zlib.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
        zlib.extend_from_slice(raw);
        zlib.extend_from_slice(&adler32(raw).to_be_bytes());
        zlib
    }

    fn adler32(bytes: &[u8]) -> u32 {
        let mut a = 1u32;
        let mut b = 0u32;
        for &byte in bytes {
            a = (a + u32::from(byte)) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }

    fn morphed_triangle_gltf_buffer() -> Vec<u8> {
        let mut bytes = triangle_gltf_buffer();
        for value in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn skinned_triangle_gltf_buffer() -> Vec<u8> {
        let mut bytes = triangle_gltf_buffer();
        bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        for _ in 0..3 {
            for value in [1.0f32, 0.0, 0.0, 0.0] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        for col in Mat4::translation([0.0, 0.0, -1.0]).to_cols_array() {
            for value in col {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }

    fn bmp_32_top_down_1x1() -> Vec<u8> {
        let pixel_data_len = 4u32;
        let file_size = 54 + pixel_data_len;
        let mut bytes = Vec::with_capacity(file_size as usize);
        bytes.extend_from_slice(b"BM");
        bytes.extend_from_slice(&file_size.to_le_bytes());
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(&54u32.to_le_bytes());
        bytes.extend_from_slice(&40u32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&(-1i32).to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&32u16.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&pixel_data_len.to_le_bytes());
        bytes.extend_from_slice(&[0; 16]);
        bytes.extend_from_slice(&[30, 20, 10, 40]);
        bytes
    }

    fn tga_32_top_left_1x1() -> Vec<u8> {
        let mut bytes = Vec::with_capacity(22);
        bytes.extend_from_slice(&[0, 0, 2]);
        bytes.extend_from_slice(&[0; 5]);
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.push(32);
        bytes.push(0x20);
        bytes.extend_from_slice(&[30, 20, 10, 40]);
        bytes
    }

    fn tga_color_mapped_top_left_1x1() -> Vec<u8> {
        let mut bytes = Vec::with_capacity(28);
        bytes.extend_from_slice(&[0, 1, 1]);
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.push(24);
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.push(8);
        bytes.push(0x20);
        bytes.extend_from_slice(&[0, 0, 255]);
        bytes.push(0);
        bytes
    }

    fn encode_base64(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut output = String::new();
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0];
            let b1 = *chunk.get(1).unwrap_or(&0);
            let b2 = *chunk.get(2).unwrap_or(&0);
            output.push(ALPHABET[(b0 >> 2) as usize] as char);
            output.push(ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
            if chunk.len() > 1 {
                output.push(ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
            } else {
                output.push('=');
            }
            if chunk.len() > 2 {
                output.push(ALPHABET[(b2 & 0x3f) as usize] as char);
            } else {
                output.push('=');
            }
        }
        output
    }

    fn triangle_glb() -> Vec<u8> {
        let json = r#"{
  "asset": { "version": "2.0" },
  "buffers": [{ "byteLength": 42 }],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
  ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
  ],
  "meshes": [{
    "primitives": [{
      "attributes": { "POSITION": 0 },
      "indices": 1
    }]
  }]
}"#;
        glb_bytes(json, &triangle_gltf_buffer())
    }

    fn glb_bytes(json_source: &str, binary_chunk: &[u8]) -> Vec<u8> {
        let mut json = json_source.as_bytes().to_vec();
        while json.len() % 4 != 0 {
            json.push(b' ');
        }

        let mut binary = binary_chunk.to_vec();
        while binary.len() % 4 != 0 {
            binary.push(0);
        }

        let total_len = 12 + 8 + json.len() + 8 + binary.len();
        let mut bytes = Vec::with_capacity(total_len);
        bytes.extend_from_slice(&0x4654_6c67u32.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&(total_len as u32).to_le_bytes());
        bytes.extend_from_slice(&(json.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0x4e4f_534au32.to_le_bytes());
        bytes.extend_from_slice(&json);
        bytes.extend_from_slice(&(binary.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0x004e_4942u32.to_le_bytes());
        bytes.extend_from_slice(&binary);
        bytes
    }
}
