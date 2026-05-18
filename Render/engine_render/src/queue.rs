use engine_graphics::Color;

use crate::{
    Camera, Mat4, MaterialHandle, Mesh, MeshHandle, MeshInstanceHandle, RenderLighting,
    RenderScene, Transform,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderPassKind {
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderDepthDesc {
    pub enabled: bool,
    pub clear_depth: f32,
}

impl RenderDepthDesc {
    pub const ENABLED: Self = Self {
        enabled: true,
        clear_depth: 1.0,
    };

    pub const DISABLED: Self = Self {
        enabled: false,
        clear_depth: 1.0,
    };

    pub const fn enabled(clear_depth: f32) -> Self {
        Self {
            enabled: true,
            clear_depth,
        }
    }

    pub const fn disabled() -> Self {
        Self::DISABLED
    }
}

impl Default for RenderDepthDesc {
    fn default() -> Self {
        Self::ENABLED
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderPassDesc {
    pub kind: RenderPassKind,
    pub camera: Camera,
    pub clear_color: Color,
    pub depth: RenderDepthDesc,
    pub lighting: RenderLighting,
    pub aspect_ratio: f32,
}

impl RenderPassDesc {
    pub fn new(kind: RenderPassKind, camera: impl Into<Camera>, clear_color: Color) -> Self {
        Self::with_depth_and_lighting(
            kind,
            camera,
            clear_color,
            RenderDepthDesc::ENABLED,
            RenderLighting::DEFAULT,
            1.0,
        )
    }

    pub fn with_depth(
        kind: RenderPassKind,
        camera: impl Into<Camera>,
        clear_color: Color,
        depth: RenderDepthDesc,
    ) -> Self {
        Self::with_depth_and_lighting(
            kind,
            camera,
            clear_color,
            depth,
            RenderLighting::DEFAULT,
            1.0,
        )
    }

    pub fn with_depth_and_lighting(
        kind: RenderPassKind,
        camera: impl Into<Camera>,
        clear_color: Color,
        depth: RenderDepthDesc,
        lighting: RenderLighting,
        aspect_ratio: f32,
    ) -> Self {
        Self {
            kind,
            camera: camera.into(),
            clear_color,
            depth,
            lighting,
            aspect_ratio: aspect_ratio.max(0.0001),
        }
    }

    pub fn main(scene: &RenderScene) -> Self {
        Self::with_depth_and_lighting(
            RenderPassKind::Main,
            scene.camera(),
            scene.clear_color(),
            scene.depth(),
            scene.lighting(),
            scene.aspect_ratio(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderItem {
    pub instance: MeshInstanceHandle,
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub transform: Transform,
    pub model_matrix: Mat4,
    pub normal_matrix: Mat4,
    pub sort_position: [f32; 3],
    pub sort_order: i32,
}

impl RenderItem {
    pub fn new(
        instance: MeshInstanceHandle,
        mesh: MeshHandle,
        material: MaterialHandle,
        transform: Transform,
        model_matrix: Mat4,
        normal_matrix: Mat4,
        sort_position: [f32; 3],
        sort_order: i32,
    ) -> Self {
        Self {
            instance,
            mesh,
            material,
            transform,
            model_matrix,
            normal_matrix,
            sort_position,
            sort_order,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderBatch {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub start: usize,
    pub end: usize,
}

impl RenderBatch {
    pub const fn new(mesh: MeshHandle, material: MaterialHandle, start: usize, end: usize) -> Self {
        Self {
            mesh,
            material,
            start,
            end,
        }
    }

    pub const fn len(self) -> usize {
        if self.end >= self.start {
            self.end - self.start
        } else {
            0
        }
    }

    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderQueueStats {
    pub item_count: usize,
    pub batch_count: usize,
    pub opaque_item_count: usize,
    pub transparent_item_count: usize,
    pub opaque_batch_count: usize,
    pub transparent_batch_count: usize,
    pub culled_item_count: usize,
    pub instance_count: usize,
    pub draw_call_count: usize,
    pub max_batch_size: usize,
}

impl RenderQueueStats {
    pub const fn saved_draw_calls(self) -> usize {
        if self.item_count >= self.draw_call_count {
            self.item_count - self.draw_call_count
        } else {
            0
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderQueue {
    pass: RenderPassDesc,
    items: Vec<RenderItem>,
    batches: Vec<RenderBatch>,
    transparent_item_start: usize,
    culled_item_count: usize,
}

impl RenderQueue {
    pub fn from_scene(scene: &RenderScene) -> Self {
        let pass = RenderPassDesc::main(scene);
        let mut opaque_items = Vec::new();
        let mut transparent_items = Vec::new();
        let mut culled_item_count = 0;

        for (handle, instance) in scene.instance_entries() {
            if !instance.visible {
                continue;
            }

            if scene.frustum_culling()
                && scene
                    .mesh(instance.mesh)
                    .and_then(Mesh::bounds)
                    .is_some_and(|bounds| {
                        !pass.camera.contains_bounds_matrix(
                            bounds,
                            instance.model_matrix(),
                            pass.aspect_ratio,
                        )
                    })
            {
                culled_item_count += 1;
                continue;
            }

            let item = RenderItem::new(
                handle,
                instance.mesh,
                instance.material,
                instance.transform,
                instance.model_matrix(),
                instance.normal_matrix(),
                instance.sort_position(),
                instance.sort_order,
            );

            if scene
                .material(instance.material)
                .map_or(false, |material| material.is_transparent())
            {
                transparent_items.push(item);
            } else {
                opaque_items.push(item);
            }
        }

        opaque_items.sort_by_key(|item| (item.sort_order, item.instance.index()));
        let camera = pass.camera;
        transparent_items.sort_by(|a, b| {
            camera
                .transparent_sort_depth(b.sort_position)
                .total_cmp(&camera.transparent_sort_depth(a.sort_position))
                .then_with(|| a.sort_order.cmp(&b.sort_order))
                .then_with(|| a.instance.index().cmp(&b.instance.index()))
        });

        let transparent_item_start = opaque_items.len();
        opaque_items.extend(transparent_items);
        let items = opaque_items;
        let batches = build_batches(&items);
        Self {
            pass,
            items,
            batches,
            transparent_item_start,
            culled_item_count,
        }
    }

    pub fn pass(&self) -> RenderPassDesc {
        self.pass
    }

    pub fn items(&self) -> &[RenderItem] {
        &self.items
    }

    pub fn batches(&self) -> &[RenderBatch] {
        &self.batches
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn stats(&self) -> RenderQueueStats {
        let mut instance_count = 0;
        let mut draw_call_count = 0;
        let mut max_batch_size = 0;
        let mut opaque_batch_count = 0;
        let mut transparent_batch_count = 0;

        for batch in &self.batches {
            let len = batch.len();
            instance_count += len;
            max_batch_size = max_batch_size.max(len);
            if len > 0 {
                draw_call_count += 1;
            }
            if batch.start >= self.transparent_item_start {
                transparent_batch_count += 1;
            } else {
                opaque_batch_count += 1;
            }
        }

        let opaque_item_count = self.transparent_item_start;
        let transparent_item_count = self.items.len().saturating_sub(self.transparent_item_start);

        RenderQueueStats {
            item_count: self.items.len(),
            batch_count: self.batches.len(),
            opaque_item_count,
            transparent_item_count,
            opaque_batch_count,
            transparent_batch_count,
            culled_item_count: self.culled_item_count,
            instance_count,
            draw_call_count,
            max_batch_size,
        }
    }
}

fn build_batches(items: &[RenderItem]) -> Vec<RenderBatch> {
    let Some(first) = items.first() else {
        return Vec::new();
    };

    let mut batches = Vec::new();
    let mut mesh = first.mesh;
    let mut material = first.material;
    let mut start = 0;

    for (index, item) in items.iter().enumerate().skip(1) {
        if item.mesh != mesh || item.material != material {
            batches.push(RenderBatch::new(mesh, material, start, index));
            mesh = item.mesh;
            material = item.material;
            start = index;
        }
    }

    batches.push(RenderBatch::new(mesh, material, start, items.len()));
    batches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Camera, Material, Mesh, OrthographicCamera, PerspectiveCamera, Texture, TextureSize,
    };

    #[test]
    fn queue_keeps_only_visible_instances_in_stable_sort_order() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let mesh = scene.add_mesh(Mesh::colored_triangle());
        let texture = scene.add_texture(Texture::white_1x1());
        let material = scene.add_material(Material::textured([1.0, 1.0, 1.0, 1.0], texture));
        let first = scene.add_instance_with_material(mesh, material, Transform::IDENTITY);
        let hidden = scene.add_instance_with_material(mesh, material, Transform::IDENTITY);
        let second = scene.add_instance_with_material(mesh, material, Transform::IDENTITY);

        scene.set_instance_sort_order(first, 10).unwrap();
        scene.set_instance_sort_order(hidden, -100).unwrap();
        scene.set_instance_sort_order(second, 10).unwrap();
        scene.set_instance_visible(hidden, false).unwrap();

        let queue = RenderQueue::from_scene(&scene);

        assert_eq!(queue.items().len(), 2);
        assert_eq!(queue.items()[0].instance, first);
        assert_eq!(queue.items()[1].instance, second);
    }

    #[test]
    fn queue_captures_pass_state_from_scene() {
        let mut scene = RenderScene::new(OrthographicCamera::new_2d(4.0));
        scene.set_clear_color(Color::rgba(0.1, 0.2, 0.3, 0.4));

        let texture = Texture::solid_rgba(TextureSize::new(1, 1), [255, 0, 0, 255]);
        scene.add_texture(texture);

        let queue = RenderQueue::from_scene(&scene);

        assert_eq!(queue.pass().kind, RenderPassKind::Main);
        assert_eq!(
            queue.pass().camera,
            Camera::Orthographic(OrthographicCamera::new_2d(4.0))
        );
        assert_eq!(queue.pass().clear_color, Color::rgba(0.1, 0.2, 0.3, 0.4));
        assert_eq!(queue.pass().depth, RenderDepthDesc::ENABLED);
        assert!(queue.is_empty());
    }

    #[test]
    fn queue_batches_consecutive_items_by_mesh_and_material() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let mesh = scene.add_mesh(Mesh::colored_triangle());
        let texture = scene.add_texture(Texture::white_1x1());
        let first_material = scene.add_material(Material::textured([1.0, 1.0, 1.0, 1.0], texture));
        let second_material = scene.add_material(Material::textured([0.5, 0.5, 1.0, 1.0], texture));
        let first = scene.add_instance_with_material(mesh, first_material, Transform::IDENTITY);
        let second = scene.add_instance_with_material(mesh, first_material, Transform::IDENTITY);
        let third = scene.add_instance_with_material(mesh, second_material, Transform::IDENTITY);
        let fourth = scene.add_instance_with_material(mesh, first_material, Transform::IDENTITY);

        scene.set_instance_sort_order(first, 0).unwrap();
        scene.set_instance_sort_order(second, 0).unwrap();
        scene.set_instance_sort_order(third, 1).unwrap();
        scene.set_instance_sort_order(fourth, 2).unwrap();

        let queue = RenderQueue::from_scene(&scene);

        assert_eq!(queue.batches().len(), 3);
        assert_eq!(
            queue.batches()[0],
            RenderBatch::new(mesh, first_material, 0, 2)
        );
        assert_eq!(
            queue.batches()[1],
            RenderBatch::new(mesh, second_material, 2, 3)
        );
        assert_eq!(
            queue.batches()[2],
            RenderBatch::new(mesh, first_material, 3, 4)
        );

        assert_eq!(
            queue.stats(),
            RenderQueueStats {
                item_count: 4,
                batch_count: 3,
                opaque_item_count: 4,
                transparent_item_count: 0,
                opaque_batch_count: 3,
                transparent_batch_count: 0,
                culled_item_count: 0,
                instance_count: 4,
                draw_call_count: 3,
                max_batch_size: 2,
            }
        );
        assert_eq!(queue.stats().saved_draw_calls(), 1);
    }

    #[test]
    fn queue_captures_explicit_instance_model_matrix() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let mesh = scene.add_mesh(Mesh::colored_triangle());
        let model_matrix = Mat4::translation([3.0, -2.0, 5.0]) * Mat4::scale([2.0, 1.0, 1.0]);
        let instance =
            scene.add_instance_with_material_matrix(mesh, scene.default_material(), model_matrix);

        let queue = RenderQueue::from_scene(&scene);

        assert_eq!(queue.items().len(), 1);
        assert_eq!(queue.items()[0].instance, instance);
        assert_eq!(queue.items()[0].model_matrix, model_matrix);
        assert_eq!(queue.items()[0].sort_position, [3.0, -2.0, 5.0]);
    }

    #[test]
    fn queue_sorts_transparent_items_back_to_front() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let mesh = scene.add_mesh(Mesh::colored_triangle());
        let texture = scene.add_texture(Texture::white_1x1());
        let material = scene.add_material(Material::alpha_blended_textured(
            [1.0, 1.0, 1.0, 0.5],
            texture,
        ));
        let near = scene.add_instance_with_material(
            mesh,
            material,
            Transform::new([0.0, 0.0, -0.5], 0.0, [1.0, 1.0, 1.0]),
        );
        let far = scene.add_instance_with_material(
            mesh,
            material,
            Transform::new([0.0, 0.0, 0.5], 0.0, [1.0, 1.0, 1.0]),
        );
        let middle = scene.add_instance_with_material(
            mesh,
            material,
            Transform::new([0.0, 0.0, 0.0], 0.0, [1.0, 1.0, 1.0]),
        );

        let queue = RenderQueue::from_scene(&scene);

        assert_eq!(
            queue
                .items()
                .iter()
                .map(|item| item.instance)
                .collect::<Vec<_>>(),
            vec![far, middle, near]
        );
        assert_eq!(queue.batches(), &[RenderBatch::new(mesh, material, 0, 3)]);
        assert_eq!(queue.stats().opaque_item_count, 0);
        assert_eq!(queue.stats().transparent_item_count, 3);
        assert_eq!(queue.stats().transparent_batch_count, 1);
    }

    #[test]
    fn queue_sorts_transparent_items_by_perspective_camera_depth() {
        let mut camera = PerspectiveCamera::default();
        camera.position = [0.0, 0.0, 4.0];
        let mut scene = RenderScene::new(camera);
        let mesh = scene.add_mesh(Mesh::colored_triangle());
        let texture = scene.add_texture(Texture::white_1x1());
        let material = scene.add_material(Material::alpha_blended_textured(
            [1.0, 1.0, 1.0, 0.5],
            texture,
        ));
        let near = scene.add_instance_with_material(
            mesh,
            material,
            Transform::new([0.0, 0.0, 0.5], 0.0, [1.0, 1.0, 1.0]),
        );
        let far = scene.add_instance_with_material(
            mesh,
            material,
            Transform::new([0.0, 0.0, -1.5], 0.0, [1.0, 1.0, 1.0]),
        );

        let queue = RenderQueue::from_scene(&scene);

        assert_eq!(
            queue
                .items()
                .iter()
                .map(|item| item.instance)
                .collect::<Vec<_>>(),
            vec![far, near]
        );
    }

    #[test]
    fn queue_places_transparent_items_after_opaque_items() {
        let mut scene = RenderScene::new(OrthographicCamera::default());
        let mesh = scene.add_mesh(Mesh::colored_triangle());
        let texture = scene.add_texture(Texture::white_1x1());
        let opaque = scene.add_material(Material::opaque_textured([1.0, 1.0, 1.0, 1.0], texture));
        let transparent = scene.add_material(Material::alpha_blended_textured(
            [1.0, 1.0, 1.0, 0.4],
            texture,
        ));
        let opaque_late = scene.add_instance_with_material(mesh, opaque, Transform::IDENTITY);
        let transparent_front = scene.add_instance_with_material(
            mesh,
            transparent,
            Transform::new([0.0, 0.0, -0.75], 0.0, [1.0, 1.0, 1.0]),
        );
        let opaque_early = scene.add_instance_with_material(mesh, opaque, Transform::IDENTITY);
        let transparent_back = scene.add_instance_with_material(
            mesh,
            transparent,
            Transform::new([0.0, 0.0, 0.75], 0.0, [1.0, 1.0, 1.0]),
        );

        scene.set_instance_sort_order(opaque_late, 10).unwrap();
        scene.set_instance_sort_order(opaque_early, -10).unwrap();
        scene
            .set_instance_sort_order(transparent_front, -100)
            .unwrap();
        scene
            .set_instance_sort_order(transparent_back, 100)
            .unwrap();

        let queue = RenderQueue::from_scene(&scene);

        assert_eq!(
            queue
                .items()
                .iter()
                .map(|item| item.instance)
                .collect::<Vec<_>>(),
            vec![
                opaque_early,
                opaque_late,
                transparent_back,
                transparent_front
            ]
        );
        assert_eq!(
            queue.batches(),
            &[
                RenderBatch::new(mesh, opaque, 0, 2),
                RenderBatch::new(mesh, transparent, 2, 4),
            ]
        );

        let stats = queue.stats();
        assert_eq!(stats.opaque_item_count, 2);
        assert_eq!(stats.transparent_item_count, 2);
        assert_eq!(stats.opaque_batch_count, 1);
        assert_eq!(stats.transparent_batch_count, 1);
    }

    #[test]
    fn queue_frustum_culling_is_opt_in() {
        let mut scene = RenderScene::new(OrthographicCamera::new_2d(2.0));
        let mesh = scene.add_mesh(Mesh::textured_quad(1.0, 1.0, [1.0, 1.0, 1.0]));
        let outside = scene.add_instance_with_material(
            mesh,
            scene.default_material(),
            Transform::new([100.0, 0.0, 0.0], 0.0, [1.0, 1.0, 1.0]),
        );

        let queue = RenderQueue::from_scene(&scene);

        assert_eq!(queue.items().len(), 1);
        assert_eq!(queue.items()[0].instance, outside);
        assert_eq!(queue.stats().culled_item_count, 0);
    }

    #[test]
    fn queue_frustum_culls_items_outside_camera() {
        let mut camera = PerspectiveCamera::default();
        camera.position = [0.0, 0.0, 4.0];
        let mut scene = RenderScene::new(camera);
        scene.set_aspect_ratio(16.0 / 9.0);
        scene.set_frustum_culling(true);
        let mesh = scene.add_mesh(Mesh::textured_cube(1.0, [1.0, 1.0, 1.0]));
        let inside = scene.add_instance_with_material(
            mesh,
            scene.default_material(),
            Transform::new_3d([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        );
        let outside = scene.add_instance_with_material(
            mesh,
            scene.default_material(),
            Transform::new_3d([100.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        );

        let queue = RenderQueue::from_scene(&scene);

        assert_eq!(
            queue
                .items()
                .iter()
                .map(|item| item.instance)
                .collect::<Vec<_>>(),
            vec![inside]
        );
        assert_eq!(queue.stats().culled_item_count, 1);
        assert!(!queue.items().iter().any(|item| item.instance == outside));
    }
}
