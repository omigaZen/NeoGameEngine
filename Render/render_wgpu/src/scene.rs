use engine_graphics::{GraphicsError, GraphicsResult};
use engine_render::{
    EnvironmentLight, Material, MaterialHandle, MeshHandle, MeshInstanceHandle, RenderQueue,
    RenderScene, Texture, TextureHandle, TextureSize,
};
use graphics_wgpu::{wgpu, WgpuGraphics, WgpuSurface};

use crate::{
    select_environment_probe_blend, EnvironmentProbeDesc, EnvironmentProbeVolume, MeshBatchDraw,
    MeshRenderer, WgpuEnvironmentProbe, WgpuEnvironmentTexture, WgpuMaterial, WgpuMesh,
    WgpuMeshInstance, WgpuTexture,
};

struct PreparedMesh {
    handle: MeshHandle,
    revision: u64,
    mesh: WgpuMesh,
}

struct PreparedTexture {
    handle: TextureHandle,
    revision: u64,
    color_texture: WgpuTexture,
    data_texture: WgpuTexture,
    environment_texture: Option<WgpuEnvironmentTexture>,
}

struct PreparedMaterial {
    handle: MaterialHandle,
    revision: u64,
    material: WgpuMaterial,
    _optical_extension_texture: Option<WgpuTexture>,
}

struct PreparedInstance {
    handle: MeshInstanceHandle,
    instance: WgpuMeshInstance,
}

pub struct WgpuRenderScene {
    meshes: Vec<Option<PreparedMesh>>,
    textures: Vec<Option<PreparedTexture>>,
    materials: Vec<Option<PreparedMaterial>>,
    instances: Vec<Option<PreparedInstance>>,
    default_color_texture: WgpuTexture,
    default_data_texture: WgpuTexture,
    default_normal_texture: WgpuTexture,
    default_emissive_texture: WgpuTexture,
    synced_mesh_revision: u64,
    synced_texture_revision: u64,
    synced_environment_texture: Option<TextureHandle>,
    synced_material_revision: u64,
    synced_instance_revision: u64,
}

impl WgpuRenderScene {
    pub fn prepare(
        graphics: &WgpuGraphics,
        renderer: &MeshRenderer,
        scene: &RenderScene,
        queue: &RenderQueue,
        aspect_ratio: f32,
    ) -> GraphicsResult<Self> {
        let mut prepared = Self {
            meshes: Vec::new(),
            textures: Vec::new(),
            materials: Vec::new(),
            instances: Vec::new(),
            default_color_texture: WgpuTexture::from_texture(graphics, &Texture::white_1x1())?,
            default_data_texture: WgpuTexture::from_texture_with_format(
                graphics,
                &Texture::white_1x1(),
                wgpu::TextureFormat::Rgba8Unorm,
            )?,
            default_normal_texture: WgpuTexture::from_texture_with_format(
                graphics,
                &Texture::solid_rgba(TextureSize::new(1, 1), [128, 128, 255, 255]),
                wgpu::TextureFormat::Rgba8Unorm,
            )?,
            default_emissive_texture: WgpuTexture::from_texture(graphics, &Texture::white_1x1())?,
            synced_mesh_revision: 0,
            synced_texture_revision: 0,
            synced_environment_texture: None,
            synced_material_revision: 0,
            synced_instance_revision: 0,
        };
        prepared.sync(graphics, renderer, scene, queue, aspect_ratio)?;
        Ok(prepared)
    }

    pub fn sync(
        &mut self,
        graphics: &WgpuGraphics,
        renderer: &MeshRenderer,
        scene: &RenderScene,
        queue: &RenderQueue,
        aspect_ratio: f32,
    ) -> GraphicsResult<()> {
        self.sync_meshes(graphics, scene)?;
        let textures_changed = self.sync_textures(graphics, scene)?;
        self.sync_materials(graphics, renderer, scene, textures_changed)?;
        self.sync_instances(graphics, renderer, scene);

        let view_projection = queue.pass().camera.view_projection(aspect_ratio);
        for item in queue.items() {
            let gpu_instance = self
                .instances
                .get_mut(item.instance.index())
                .and_then(Option::as_mut)
                .ok_or_else(|| {
                    GraphicsError::InvalidResource(format!(
                        "prepared render scene is missing instance {}:{}",
                        item.instance.index(),
                        item.instance.generation()
                    ))
                })?;
            if gpu_instance.handle != item.instance {
                return Err(GraphicsError::InvalidResource(format!(
                    "prepared render scene instance {}:{} does not match queue instance {}:{}",
                    gpu_instance.handle.index(),
                    gpu_instance.handle.generation(),
                    item.instance.index(),
                    item.instance.generation()
                )));
            }

            let model = item.model_matrix;
            let normal_matrix = item.normal_matrix;
            let matrix = view_projection * model;
            gpu_instance
                .instance
                .set_model_view_projection_normal_and_model_matrix(
                    graphics,
                    matrix,
                    normal_matrix,
                    model,
                );
        }

        Ok(())
    }

    fn sync_textures(
        &mut self,
        graphics: &WgpuGraphics,
        scene: &RenderScene,
    ) -> GraphicsResult<bool> {
        let environment_texture = scene.lighting().environment.texture;
        if self.synced_texture_revision == scene.texture_revision_id()
            && self.textures.len() == scene.texture_slot_len()
            && self.synced_environment_texture == environment_texture
        {
            return Ok(false);
        }

        self.textures.resize_with(scene.texture_slot_len(), || None);
        let mut seen = vec![false; scene.texture_slot_len()];

        for (handle, texture, revision) in scene.texture_entries() {
            seen[handle.index()] = true;
            let slot = &mut self.textures[handle.index()];
            let needs_environment_texture = environment_texture == Some(handle);
            let needs_rebuild = slot.as_ref().map_or(true, |prepared| {
                prepared.handle != handle
                    || prepared.revision != revision
                    || prepared.environment_texture.is_some() != needs_environment_texture
            });

            if needs_rebuild {
                let prepared_environment = if needs_environment_texture {
                    Some(WgpuTexture::from_environment_texture(graphics, texture)?)
                } else {
                    None
                };
                *slot = Some(PreparedTexture {
                    handle,
                    revision,
                    color_texture: WgpuTexture::from_texture(graphics, texture)?,
                    data_texture: WgpuTexture::from_texture_with_format(
                        graphics,
                        texture,
                        wgpu::TextureFormat::Rgba8Unorm,
                    )?,
                    environment_texture: prepared_environment,
                });
            }
        }

        clear_unseen_slots(&mut self.textures, &seen);
        self.synced_texture_revision = scene.texture_revision_id();
        self.synced_environment_texture = environment_texture;
        Ok(true)
    }

    fn sync_materials(
        &mut self,
        graphics: &WgpuGraphics,
        renderer: &MeshRenderer,
        scene: &RenderScene,
        force_rebuild: bool,
    ) -> GraphicsResult<()> {
        if self.synced_material_revision == scene.material_revision_id()
            && self.materials.len() == scene.material_slot_len()
            && !force_rebuild
        {
            return Ok(());
        }

        self.materials
            .resize_with(scene.material_slot_len(), || None);
        let mut seen = vec![false; scene.material_slot_len()];

        for (handle, material, revision) in scene.material_entries() {
            seen[handle.index()] = true;
            let base_color_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.base_color_texture,
                "base color",
            )?
            .map_or(&self.default_color_texture, |prepared| {
                &prepared.color_texture
            });
            let metallic_roughness_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.metallic_roughness_texture,
                "metallic-roughness",
            )?
            .map_or(&self.default_data_texture, |prepared| {
                &prepared.data_texture
            });
            let normal_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.normal_texture,
                "normal",
            )?
            .map_or(&self.default_normal_texture, |prepared| {
                &prepared.data_texture
            });
            let emissive_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.emissive_texture,
                "emissive",
            )?
            .map_or(&self.default_emissive_texture, |prepared| {
                &prepared.color_texture
            });
            let occlusion_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.occlusion_texture,
                "occlusion",
            )?
            .map_or(&self.default_data_texture, |prepared| {
                &prepared.data_texture
            });
            let clearcoat_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.clearcoat_texture,
                "clearcoat",
            )?
            .map_or(&self.default_data_texture, |prepared| {
                &prepared.data_texture
            });
            let clearcoat_roughness_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.clearcoat_roughness_texture,
                "clearcoat roughness",
            )?
            .map_or(&self.default_data_texture, |prepared| {
                &prepared.data_texture
            });
            let clearcoat_normal_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.clearcoat_normal_texture,
                "clearcoat normal",
            )?
            .map_or(&self.default_normal_texture, |prepared| {
                &prepared.data_texture
            });
            let sheen_color_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.sheen_color_texture,
                "sheen color",
            )?
            .map_or(&self.default_color_texture, |prepared| {
                &prepared.color_texture
            });
            let sheen_roughness_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.sheen_roughness_texture,
                "sheen roughness",
            )?
            .map_or(&self.default_data_texture, |prepared| {
                &prepared.data_texture
            });
            let transmission_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.transmission_texture,
                "transmission",
            )?
            .map_or(&self.default_data_texture, |prepared| {
                &prepared.data_texture
            });
            let specular_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.specular_texture,
                "specular",
            )?
            .map_or(&self.default_data_texture, |prepared| {
                &prepared.data_texture
            });
            let specular_color_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.specular_color_texture,
                "specular color",
            )?
            .map_or(&self.default_color_texture, |prepared| {
                &prepared.color_texture
            });
            let anisotropy_texture = prepared_texture(
                &self.textures,
                scene,
                handle,
                material.anisotropy_texture,
                "anisotropy",
            )?
            .map_or(&self.default_data_texture, |prepared| {
                &prepared.data_texture
            });
            let packed_optical_texture = build_optical_extension_texture(scene, handle, material)?;
            let optical_extension_texture = packed_optical_texture
                .as_ref()
                .map(|texture| {
                    WgpuTexture::from_texture_with_format(
                        graphics,
                        texture,
                        wgpu::TextureFormat::Rgba8Unorm,
                    )
                })
                .transpose()?;
            let optical_extension_texture_ref = optical_extension_texture
                .as_ref()
                .unwrap_or(&self.default_data_texture);

            let slot = &mut self.materials[handle.index()];
            let needs_rebuild = force_rebuild
                || slot.as_ref().map_or(true, |prepared| {
                    prepared.handle != handle || prepared.revision != revision
                });

            if needs_rebuild {
                *slot = Some(PreparedMaterial {
                    handle,
                    revision,
                    material: renderer.create_material(
                        graphics,
                        *material,
                        base_color_texture,
                        metallic_roughness_texture,
                        normal_texture,
                        emissive_texture,
                        occlusion_texture,
                        clearcoat_texture,
                        clearcoat_roughness_texture,
                        clearcoat_normal_texture,
                        sheen_color_texture,
                        sheen_roughness_texture,
                        transmission_texture,
                        specular_texture,
                        specular_color_texture,
                        anisotropy_texture,
                        optical_extension_texture_ref,
                    ),
                    _optical_extension_texture: optical_extension_texture,
                });
            }
        }

        clear_unseen_slots(&mut self.materials, &seen);
        self.synced_material_revision = scene.material_revision_id();
        Ok(())
    }

    fn sync_meshes(&mut self, graphics: &WgpuGraphics, scene: &RenderScene) -> GraphicsResult<()> {
        if self.synced_mesh_revision == scene.mesh_revision_id()
            && self.meshes.len() == scene.mesh_slot_len()
        {
            return Ok(());
        }

        self.meshes.resize_with(scene.mesh_slot_len(), || None);
        let mut seen = vec![false; scene.mesh_slot_len()];

        for (handle, mesh, revision) in scene.mesh_entries() {
            seen[handle.index()] = true;
            let slot = &mut self.meshes[handle.index()];
            let needs_rebuild = slot.as_ref().map_or(true, |prepared| {
                prepared.handle != handle || prepared.revision != revision
            });

            if needs_rebuild {
                *slot = Some(PreparedMesh {
                    handle,
                    revision,
                    mesh: WgpuMesh::from_mesh(graphics, mesh)?,
                });
            }
        }

        clear_unseen_slots(&mut self.meshes, &seen);
        self.synced_mesh_revision = scene.mesh_revision_id();
        Ok(())
    }

    fn sync_instances(
        &mut self,
        graphics: &WgpuGraphics,
        renderer: &MeshRenderer,
        scene: &RenderScene,
    ) {
        if self.synced_instance_revision == scene.instance_revision_id()
            && self.instances.len() == scene.instance_slot_len()
        {
            return;
        }

        self.instances
            .resize_with(scene.instance_slot_len(), || None);
        let mut seen = vec![false; scene.instance_slot_len()];

        for (handle, _) in scene.instance_entries() {
            seen[handle.index()] = true;
            let needs_rebuild = self.instances[handle.index()]
                .as_ref()
                .map_or(true, |prepared| prepared.handle != handle);

            if needs_rebuild {
                self.instances[handle.index()] = Some(PreparedInstance {
                    handle,
                    instance: renderer.create_instance(graphics, engine_render::Mat4::IDENTITY),
                });
            }
        }

        clear_unseen_slots(&mut self.instances, &seen);
        self.synced_instance_revision = scene.instance_revision_id();
    }

    pub fn render(
        &self,
        renderer: &mut MeshRenderer,
        surface: &mut WgpuSurface,
        queue: &RenderQueue,
    ) -> GraphicsResult<()> {
        self.render_with_environment_texture(renderer, surface, queue, None)
    }

    pub fn render_with_environment_probe(
        &self,
        renderer: &mut MeshRenderer,
        surface: &mut WgpuSurface,
        queue: &RenderQueue,
        probe: &WgpuEnvironmentProbe,
    ) -> GraphicsResult<()> {
        self.render_with_environment_texture(
            renderer,
            surface,
            queue,
            Some(probe.environment_texture()),
        )
    }

    pub fn render_with_environment_probe_volumes(
        &self,
        renderer: &mut MeshRenderer,
        surface: &mut WgpuSurface,
        queue: &RenderQueue,
        volumes: &[EnvironmentProbeVolume<'_>],
    ) -> GraphicsResult<()> {
        let pass = queue.pass();
        let probes = select_environment_probe_blend(pass.camera.position(), volumes);
        self.render_with_environment_probe_blend(renderer, surface, queue, &probes)
    }

    pub fn render_with_environment_probe_blend(
        &self,
        renderer: &mut MeshRenderer,
        surface: &mut WgpuSurface,
        queue: &RenderQueue,
        probes: &[crate::EnvironmentProbeBlend<'_>],
    ) -> GraphicsResult<()> {
        let pass = queue.pass();
        renderer.set_clear_color(pass.clear_color);
        renderer.set_depth(pass.depth);
        renderer.set_lighting(pass.lighting);
        renderer.set_camera_position(pass.camera.position());
        renderer.set_shadow_camera(pass.camera, pass.aspect_ratio);
        let prepared_environment_texture =
            prepared_environment_texture(&self.textures, &pass.lighting.environment)?;
        let draws = self.build_draws(queue)?;

        renderer.render_batches_with_environment_probes(
            surface,
            &draws,
            prepared_environment_texture,
            probes,
        )
    }

    pub fn render_with_environment_texture(
        &self,
        renderer: &mut MeshRenderer,
        surface: &mut WgpuSurface,
        queue: &RenderQueue,
        environment_texture_override: Option<&WgpuEnvironmentTexture>,
    ) -> GraphicsResult<()> {
        let pass = queue.pass();
        renderer.set_clear_color(pass.clear_color);
        renderer.set_depth(pass.depth);
        renderer.set_lighting(pass.lighting);
        renderer.set_camera_position(pass.camera.position());
        renderer.set_shadow_camera(pass.camera, pass.aspect_ratio);
        let prepared_environment_texture =
            prepared_environment_texture(&self.textures, &pass.lighting.environment)?;
        let environment_texture = environment_texture_override.or(prepared_environment_texture);
        let draws = self.build_draws(queue)?;

        renderer.render_batches_with_environment(surface, &draws, environment_texture)
    }

    pub fn capture_environment_probe(
        &self,
        renderer: &mut MeshRenderer,
        graphics: &WgpuGraphics,
        probe: &mut WgpuEnvironmentProbe,
        queue: &RenderQueue,
        desc: EnvironmentProbeDesc,
    ) -> GraphicsResult<()> {
        let pass = queue.pass();
        renderer.set_lighting(pass.lighting);
        let draws = self.build_draws(queue)?;

        renderer.capture_environment_probe(graphics, probe, desc, &draws)
    }

    fn build_draws<'a>(&'a self, queue: &RenderQueue) -> GraphicsResult<Vec<MeshBatchDraw<'a>>> {
        let mut draws = Vec::with_capacity(queue.batches().len());
        for batch in queue.batches() {
            let mesh = self
                .meshes
                .get(batch.mesh.index())
                .and_then(Option::as_ref)
                .filter(|prepared| prepared.handle == batch.mesh)
                .ok_or_else(|| {
                    GraphicsError::InvalidResource(format!(
                        "scene references missing prepared mesh {}:{}",
                        batch.mesh.index(),
                        batch.mesh.generation()
                    ))
                })?;
            let material = self
                .materials
                .get(batch.material.index())
                .and_then(Option::as_ref)
                .filter(|prepared| prepared.handle == batch.material)
                .ok_or_else(|| {
                    GraphicsError::InvalidResource(format!(
                        "scene references missing prepared material {}:{}",
                        batch.material.index(),
                        batch.material.generation()
                    ))
                })?;

            let items = queue.items().get(batch.start..batch.end).ok_or_else(|| {
                GraphicsError::InvalidResource(format!(
                    "render batch references invalid item range {}..{} for {} items",
                    batch.start,
                    batch.end,
                    queue.len()
                ))
            })?;
            let mut instances = Vec::with_capacity(batch.len());

            for item in items {
                if item.mesh != batch.mesh || item.material != batch.material {
                    return Err(GraphicsError::InvalidResource(format!(
                        "render batch {}:{} / {}:{} contains item {}:{} / {}:{}",
                        batch.mesh.index(),
                        batch.mesh.generation(),
                        batch.material.index(),
                        batch.material.generation(),
                        item.mesh.index(),
                        item.mesh.generation(),
                        item.material.index(),
                        item.material.generation()
                    )));
                }

                let gpu_instance = self
                    .instances
                    .get(item.instance.index())
                    .and_then(Option::as_ref)
                    .filter(|prepared| prepared.handle == item.instance)
                    .ok_or_else(|| {
                        GraphicsError::InvalidResource(format!(
                            "scene references missing prepared instance {}:{}",
                            item.instance.index(),
                            item.instance.generation()
                        ))
                    })?;
                instances.push(&gpu_instance.instance);
            }

            if instances.is_empty() {
                return Err(GraphicsError::InvalidResource(format!(
                    "render batch references empty item range {}..{}",
                    batch.start, batch.end
                )));
            }

            draws.push(MeshBatchDraw::new(
                &mesh.mesh,
                &material.material,
                instances,
            ));
        }

        Ok(draws)
    }
}

fn clear_unseen_slots<T>(slots: &mut [Option<T>], seen: &[bool]) {
    for (index, slot) in slots.iter_mut().enumerate() {
        if !seen.get(index).copied().unwrap_or(false) {
            *slot = None;
        }
    }
}

fn build_optical_extension_texture(
    scene: &RenderScene,
    material_handle: MaterialHandle,
    material: &Material,
) -> GraphicsResult<Option<Texture>> {
    let iridescence_texture = scene_texture(
        scene,
        material_handle,
        material.iridescence_texture,
        "iridescence",
    )?;
    let iridescence_thickness_texture = scene_texture(
        scene,
        material_handle,
        material.iridescence_thickness_texture,
        "iridescence thickness",
    )?;
    let thickness_texture = scene_texture(
        scene,
        material_handle,
        material.thickness_texture,
        "volume thickness",
    )?;

    if iridescence_texture.is_none()
        && iridescence_thickness_texture.is_none()
        && thickness_texture.is_none()
    {
        return Ok(None);
    }

    let target_size = [
        iridescence_texture,
        iridescence_thickness_texture,
        thickness_texture,
    ]
    .into_iter()
    .flatten()
    .map(Texture::size)
    .fold(TextureSize::new(1, 1), |size, texture_size| {
        TextureSize::new(
            size.width.max(texture_size.width),
            size.height.max(texture_size.height),
        )
    });
    let mut rgba8 = Vec::with_capacity(target_size.byte_len().unwrap_or(0));

    for y in 0..target_size.height {
        for x in 0..target_size.width {
            rgba8.push(sample_texture_channel(
                iridescence_texture,
                target_size,
                x,
                y,
                0,
                255,
            ));
            rgba8.push(sample_texture_channel(
                iridescence_thickness_texture,
                target_size,
                x,
                y,
                1,
                255,
            ));
            rgba8.push(sample_texture_channel(
                thickness_texture,
                target_size,
                x,
                y,
                1,
                255,
            ));
            rgba8.push(255);
        }
    }

    Texture::rgba8(target_size, rgba8)
        .ok_or_else(|| GraphicsError::InvalidResource("packed optical texture is invalid".into()))
        .map(Some)
}

fn scene_texture<'a>(
    scene: &'a RenderScene,
    material_handle: MaterialHandle,
    texture_handle: Option<TextureHandle>,
    channel: &str,
) -> GraphicsResult<Option<&'a Texture>> {
    let Some(texture_handle) = texture_handle else {
        return Ok(None);
    };

    scene
        .texture(texture_handle)
        .ok_or_else(|| {
            GraphicsError::InvalidResource(format!(
                "material {}:{} references stale {channel} texture {}:{}",
                material_handle.index(),
                material_handle.generation(),
                texture_handle.index(),
                texture_handle.generation()
            ))
        })
        .map(Some)
}

fn sample_texture_channel(
    texture: Option<&Texture>,
    target_size: TextureSize,
    target_x: u32,
    target_y: u32,
    channel: usize,
    default: u8,
) -> u8 {
    let Some(texture) = texture else {
        return default;
    };
    let source_size = texture.size();
    let source_x = ((u64::from(target_x) * u64::from(source_size.width))
        / u64::from(target_size.width))
    .min(u64::from(source_size.width.saturating_sub(1))) as u32;
    let source_y = ((u64::from(target_y) * u64::from(source_size.height))
        / u64::from(target_size.height))
    .min(u64::from(source_size.height.saturating_sub(1))) as u32;
    let offset = ((source_y * source_size.width + source_x) * 4) as usize + channel;

    texture.rgba8_data().get(offset).copied().unwrap_or(default)
}

fn prepared_texture<'a>(
    textures: &'a [Option<PreparedTexture>],
    scene: &RenderScene,
    material_handle: MaterialHandle,
    texture_handle: Option<TextureHandle>,
    channel: &str,
) -> GraphicsResult<Option<&'a PreparedTexture>> {
    let Some(texture_handle) = texture_handle else {
        return Ok(None);
    };

    scene.texture(texture_handle).ok_or_else(|| {
        GraphicsError::InvalidResource(format!(
            "material {}:{} references stale {channel} texture {}:{}",
            material_handle.index(),
            material_handle.generation(),
            texture_handle.index(),
            texture_handle.generation()
        ))
    })?;

    textures
        .get(texture_handle.index())
        .and_then(Option::as_ref)
        .filter(|prepared| prepared.handle == texture_handle)
        .ok_or_else(|| {
            GraphicsError::InvalidResource(format!(
                "material {}:{} references missing prepared {channel} texture {}:{}",
                material_handle.index(),
                material_handle.generation(),
                texture_handle.index(),
                texture_handle.generation()
            ))
        })
        .map(Some)
}

fn prepared_environment_texture<'a>(
    textures: &'a [Option<PreparedTexture>],
    environment: &EnvironmentLight,
) -> GraphicsResult<Option<&'a WgpuEnvironmentTexture>> {
    let Some(texture_handle) = environment.texture else {
        return Ok(None);
    };

    let prepared = textures
        .get(texture_handle.index())
        .and_then(Option::as_ref)
        .filter(|prepared| prepared.handle == texture_handle)
        .ok_or_else(|| {
            GraphicsError::InvalidResource(format!(
                "lighting references missing environment texture {}:{}",
                texture_handle.index(),
                texture_handle.generation()
            ))
        })?;

    let environment_texture = prepared.environment_texture.as_ref().ok_or_else(|| {
            GraphicsError::InvalidResource(format!(
                "lighting environment texture {}:{} was not prepared as an environment texture",
                texture_handle.index(),
                texture_handle.generation()
            ))
        })?;

    Ok(Some(environment_texture))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_render::OrthographicCamera;

    #[test]
    fn optical_extension_texture_packs_spec_channels() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let iridescence = scene.add_texture(
            Texture::rgba8(TextureSize::new(2, 1), [10, 20, 30, 255, 40, 50, 60, 255]).unwrap(),
        );
        let iridescence_thickness = scene.add_texture(
            Texture::rgba8(
                TextureSize::new(2, 1),
                [70, 80, 90, 255, 100, 110, 120, 255],
            )
            .unwrap(),
        );
        let thickness = scene
            .add_texture(Texture::rgba8(TextureSize::new(1, 1), [130, 140, 150, 255]).unwrap());
        let material = Material::new([1.0; 4])
            .with_iridescence_texture(iridescence)
            .with_iridescence_thickness_texture(iridescence_thickness)
            .with_thickness_texture(thickness);
        let material_handle = scene.add_material(material);
        let material = scene.material(material_handle).unwrap();

        let packed = build_optical_extension_texture(&scene, material_handle, material)
            .unwrap()
            .unwrap();

        assert_eq!(packed.size(), TextureSize::new(2, 1));
        assert_eq!(packed.rgba8_data(), &[10, 80, 140, 255, 40, 110, 140, 255]);
    }

    #[test]
    fn optical_extension_texture_uses_white_defaults_for_missing_maps() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let thickness =
            scene.add_texture(Texture::rgba8(TextureSize::new(1, 1), [10, 20, 30, 255]).unwrap());
        let material = Material::new([1.0; 4]).with_thickness_texture(thickness);
        let material_handle = scene.add_material(material);
        let material = scene.material(material_handle).unwrap();

        let packed = build_optical_extension_texture(&scene, material_handle, material)
            .unwrap()
            .unwrap();

        assert_eq!(packed.rgba8_data(), &[255, 255, 20, 255]);
    }
}
