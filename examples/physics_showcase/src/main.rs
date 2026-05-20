use engine_physics::prelude::*;

fn main() -> PhysicsResult<()> {
    let mut physics = PhysicsWorld::new(PhysicsConfig::default());

    let ground = physics.create_body(
        BodyDesc::fixed()
            .with_translation(Vec3::new(0.0, -1.0, 0.0))
            .with_debug_name("Ground"),
    )?;
    physics.create_collider_with_parent(
        ground,
        ColliderDesc::cuboid(Vec3::new(10.0, 1.0, 10.0)).with_debug_name("Ground Collider"),
    )?;

    let cube = physics.create_body(
        BodyDesc::dynamic()
            .with_translation(Vec3::new(0.0, 3.0, 0.0))
            .with_debug_name("Falling Cube"),
    )?;
    let cube_collider =
        physics.create_collider_with_parent(cube, ColliderDesc::cuboid(Vec3::splat(0.5)))?;

    for _ in 0..120 {
        physics.update_fixed(1.0 / 60.0);
    }

    let hit = physics.query().cast_ray(
        Ray {
            origin: Vec3::new(0.0, 5.0, 0.0),
            direction: Vec3::new(0.0, -1.0, 0.0),
            max_toi: 20.0,
        },
        QueryFilter::default(),
    );
    assert!(hit.is_some());

    let controller = physics.create_character_controller(CharacterControllerDesc::default());
    let character = physics
        .create_body(BodyDesc::kinematic_position().with_translation(Vec3::new(2.0, 1.0, 0.0)))?;
    let character_collider = physics
        .create_collider_with_parent(character, ColliderDesc::capsule(Axis3::Y, 0.5, 0.25))?;
    let _move = physics.move_character(CharacterMoveInput {
        controller,
        body: character,
        collider: character_collider,
        desired_translation: Vec3::new(0.25, 0.0, 0.0),
        dt: 1.0 / 60.0,
        filter: QueryFilter::default(),
    })?;

    let snapshot = physics.snapshot();
    physics.restore(snapshot)?;

    let mut transforms = [(
        cube,
        Transform::IDENTITY,
        PhysicsSyncComponent {
            mode: PhysicsSyncMode::PhysicsToTransform,
            interpolate: true,
        },
    )];
    let _ = physics.sync_transforms_from_physics(&mut transforms);

    let mut debug = DebugCollector::default();
    physics.debug_draw(&mut debug, PhysicsDebugDrawOptions::default());

    let mut backend = DefaultPhysicsBackend::try_new(PhysicsConfig::default())?;
    let mut backend_events = Vec::new();
    let _ = backend.step(1.0 / 60.0, &mut backend_events);

    let _events: Vec<_> = physics.drain_events().collect();
    assert!(physics.contains_body(cube));
    assert!(physics.contains_collider(cube_collider));
    Ok(())
}
