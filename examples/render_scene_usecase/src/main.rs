use engine_render::{
    Material, Mesh, OrthographicCamera, RenderQueue, RenderScene, Texture, TextureSize, Transform,
};

fn main() {
    let mut scene = RenderScene::new(OrthographicCamera::new_2d(2.0));
    let quad = scene.add_mesh(demo_quad_mesh());
    let checker = scene.add_texture(Texture::checkerboard_rgba8(
        TextureSize::new(16, 16),
        4,
        [255, 255, 255, 255],
        [32, 48, 64, 255],
    ));
    let warm = scene.add_material(Material::textured([1.0, 0.72, 0.55, 1.0], checker));
    let cool = scene.add_material(Material::textured([0.52, 0.82, 1.0, 1.0], checker));
    let accent = scene.add_material(Material::textured([0.7, 1.0, 0.65, 1.0], checker));

    let back = scene.add_instance_with_material(
        quad,
        warm,
        Transform::new([-0.55, 0.0, 0.0], -0.15, [0.65, 0.65, 1.0]),
    );
    let hidden = scene.add_instance_with_material(
        quad,
        cool,
        Transform::new([0.0, 0.0, 0.0], 0.0, [0.55, 0.55, 1.0]),
    );
    let front = scene.add_instance_with_material(
        quad,
        accent,
        Transform::new([0.48, 0.0, 0.0], 0.2, [0.45, 0.45, 1.0]),
    );
    let front_pair = scene.add_instance_with_material(
        quad,
        accent,
        Transform::new([0.18, 0.36, 0.0], -0.3, [0.28, 0.28, 1.0]),
    );
    let imported = scene
        .add_obj_mtl_instances_with_textures(
            demo_material_obj(),
            demo_material_mtl(),
            Transform::new([0.0, -0.45, 0.0], 0.0, [0.35, 0.35, 1.0]),
            demo_texture,
        )
        .expect("embedded OBJ/MTL asset should import");
    assert_eq!(imported.len(), 2);
    assert_eq!(
        scene
            .material(imported[1].material)
            .map(|material| material.tint[3]),
        Some(0.55)
    );
    assert!(imported.iter().all(|part| scene
        .material(part.material)
        .and_then(|material| material.base_color_texture)
        .is_some()));
    for part in &imported {
        scene.set_instance_visible(part.instance, false).unwrap();
    }

    scene.set_instance_sort_order(back, 20).unwrap();
    scene.set_instance_sort_order(hidden, 10).unwrap();
    scene.set_instance_sort_order(front, -5).unwrap();
    scene.set_instance_sort_order(front_pair, -5).unwrap();
    scene.set_instance_visible(hidden, false).unwrap();

    let transform_only_revision = scene.instance_revision_id();
    scene
        .set_instance_transform(
            front,
            Transform::new([0.55, 0.08, 0.0], 0.35, [0.5, 0.5, 1.0]),
        )
        .unwrap();
    assert_eq!(scene.instance_revision_id(), transform_only_revision);

    let temporary = scene.add_material(Material::new([1.0, 0.0, 1.0, 1.0]));
    let temporary_index = temporary.index();
    scene.remove_material(temporary).unwrap();
    let reused = scene.add_material(Material::new([0.2, 0.2, 0.2, 1.0]));

    assert_eq!(reused.index(), temporary_index);
    assert_ne!(reused.generation(), temporary.generation());
    assert!(scene.material(temporary).is_none());
    assert!(scene.material(reused).is_some());

    let queue = RenderQueue::from_scene(&scene);
    assert_eq!(
        queue
            .items()
            .iter()
            .map(|item| item.instance)
            .collect::<Vec<_>>(),
        vec![front, front_pair, back]
    );
    assert_eq!(queue.batches().len(), 2);
    assert_eq!(queue.batches()[0].start, 0);
    assert_eq!(queue.batches()[0].end, 2);
    assert_eq!(queue.batches()[1].start, 2);
    assert_eq!(queue.batches()[1].end, 3);
    assert_eq!(queue.stats().draw_call_count, 2);
    assert_eq!(queue.stats().saved_draw_calls(), 1);

    println!("RenderScene use case");
    println!(
        "  meshes: {} slots / {} active",
        scene.mesh_slot_len(),
        scene.mesh_entries().count()
    );
    println!(
        "  textures: {} slots / {} active",
        scene.texture_slot_len(),
        scene.texture_entries().count()
    );
    println!(
        "  materials: {} slots / {} active",
        scene.material_slot_len(),
        scene.material_entries().count()
    );
    println!(
        "  instances: {} slots / {} active",
        scene.instance_slot_len(),
        scene.instance_count()
    );
    println!(
        "  render pass: {:?} clear=({:.2}, {:.2}, {:.2}, {:.2}) depth_enabled={} clear_depth={:.2}",
        queue.pass().kind,
        queue.pass().clear_color.r,
        queue.pass().clear_color.g,
        queue.pass().clear_color.b,
        queue.pass().clear_color.a,
        queue.pass().depth.enabled,
        queue.pass().depth.clear_depth
    );
    println!(
        "  reused material slot: old {}:{} -> new {}:{}",
        temporary.index(),
        temporary.generation(),
        reused.index(),
        reused.generation()
    );
    println!("  visible draw order:");
    for (index, item) in queue.items().iter().enumerate() {
        println!(
            "    {index}: instance {}:{} sort={} material {}:{}",
            item.instance.index(),
            item.instance.generation(),
            item.sort_order,
            item.material.index(),
            item.material.generation()
        );
    }
    let stats = queue.stats();
    println!(
        "  queue stats: items={} batches={} instances={} draw_calls={} saved_draw_calls={} max_batch={}",
        stats.item_count,
        stats.batch_count,
        stats.instance_count,
        stats.draw_call_count,
        stats.saved_draw_calls(),
        stats.max_batch_size
    );
    println!("  render batches:");
    for (index, batch) in queue.batches().iter().enumerate() {
        println!(
            "    {index}: items {}..{} mesh {}:{} material {}:{} draws={}",
            batch.start,
            batch.end,
            batch.mesh.index(),
            batch.mesh.generation(),
            batch.material.index(),
            batch.material.generation(),
            batch.len()
        );
    }
    println!("  ok");
}

fn demo_quad_mesh() -> Mesh {
    Mesh::from_obj_str(
        "\
v -0.5 0.5 0.0
v -0.5 -0.5 0.0
v 0.5 -0.5 0.0
v 0.5 0.5 0.0
vt 0.0 0.0
vt 0.0 1.0
vt 1.0 1.0
vt 1.0 0.0
vn 0.0 0.0 1.0
f 1/1/1 2/2/1 3/3/1 4/4/1
",
    )
    .expect("embedded OBJ quad should parse")
}

fn demo_material_obj() -> &'static str {
    "\
v -0.5 0.0 0.0
v 0.0 -0.5 0.0
v 0.5 0.0 0.0
v 0.0 0.5 0.0
usemtl brushed
f 1 2 3
usemtl translucent
f 1 3 4
"
}

fn demo_material_mtl() -> &'static str {
    "\
newmtl brushed
Kd 0.9 0.72 0.42
Pr 0.24
Pm 0.45
map_Kd brushed_checker.bmp
newmtl translucent
Kd 0.45 0.75 1.0
d 0.55
Pr 0.82
map_Kd translucent_checker.bmp
"
}

fn demo_texture(path: &str) -> Option<Texture> {
    let bytes = match path {
        "brushed_checker.bmp" => Some(demo_bmp_2x2([
            [232, 184, 96, 255],
            [110, 84, 48, 255],
            [110, 84, 48, 255],
            [232, 184, 96, 255],
        ])),
        "translucent_checker.bmp" => Some(demo_bmp_2x2([
            [96, 160, 255, 180],
            [30, 70, 130, 180],
            [30, 70, 130, 180],
            [96, 160, 255, 180],
        ])),
        _ => None,
    }?;

    Texture::from_image_bytes(path, &bytes).ok()
}

fn demo_bmp_2x2(pixels: [[u8; 4]; 4]) -> Vec<u8> {
    let pixel_data_len = 16u32;
    let file_size = 54 + pixel_data_len;
    let mut bytes = Vec::with_capacity(file_size as usize);
    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&file_size.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    bytes.extend_from_slice(&54u32.to_le_bytes());
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&2i32.to_le_bytes());
    bytes.extend_from_slice(&(-2i32).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&pixel_data_len.to_le_bytes());
    bytes.extend_from_slice(&[0; 16]);

    for [red, green, blue, alpha] in pixels {
        bytes.extend_from_slice(&[blue, green, red, alpha]);
    }

    bytes
}
