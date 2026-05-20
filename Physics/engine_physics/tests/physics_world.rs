use engine_physics::prelude::*;

fn world_no_sleep() -> PhysicsWorld {
    let mut config = PhysicsConfig::default();
    config.sleeping.enabled = false;
    PhysicsWorld::new(config)
}

fn add_ground(world: &mut PhysicsWorld) -> PhysicsResult<(BodyId, ColliderId)> {
    let ground =
        world.create_body(BodyDesc::fixed().with_translation(Vec3::new(0.0, -1.0, 0.0)))?;
    let ground_collider = world
        .create_collider_with_parent(ground, ColliderDesc::cuboid(Vec3::new(5.0, 1.0, 5.0)))?;
    Ok((ground, ground_collider))
}

#[test]
fn fixed_step_moves_dynamic_body_and_reports_collision_contacts() -> PhysicsResult<()> {
    let mut world = world_no_sleep();
    let (_, ground_collider) = add_ground(&mut world)?;
    let body = world.create_body(BodyDesc::dynamic().with_translation(Vec3::new(0.0, 2.0, 0.0)))?;
    let collider =
        world.create_collider_with_parent(body, ColliderDesc::cuboid(Vec3::splat(0.5)))?;

    for _ in 0..90 {
        world.step_fixed(1.0 / 60.0);
    }

    let transform = world.body_transform(body)?;
    assert!(transform.translation.y >= 0.49);
    assert!(world
        .body_previous_transform(body)?
        .translation
        .y
        .is_finite());
    assert!(world
        .body_interpolated_transform(body, 0.5)?
        .translation
        .y
        .is_finite());
    assert!(world.events().iter().any(|event| matches!(
        event,
        PhysicsEvent::CollisionStarted(e)
            if (e.a == collider && e.b == ground_collider)
                || (e.a == ground_collider && e.b == collider)
    )));
    assert!(world.contact_pair(collider, ground_collider)?.is_some());
    assert!(world
        .events()
        .iter()
        .any(|event| matches!(event, PhysicsEvent::ContactForce(_))));
    Ok(())
}

#[test]
fn lifecycle_recursive_destroy_and_generation_mismatch_are_reported() -> PhysicsResult<()> {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());
    let parent = world.create_body(BodyDesc::dynamic())?;
    let child = world.create_body(BodyDesc::dynamic().with_translation(Vec3::X))?;
    let collider = world.create_collider_with_parent(parent, ColliderDesc::sphere(0.5))?;
    let joint = world.create_joint(parent, child, JointDesc::distance(0.0, 2.0))?;

    let destroyed = world.destroy_body_recursive(parent)?;
    assert_eq!(destroyed.bodies, vec![parent]);
    assert_eq!(destroyed.colliders, vec![collider]);
    assert_eq!(destroyed.joints, vec![joint]);
    assert!(!world.contains_body(parent));
    assert!(matches!(
        world.body_transform(parent),
        Err(PhysicsError::BodyNotFound(_))
    ));
    assert!(matches!(
        world.collider_shape(collider),
        Err(PhysicsError::ColliderNotFound(_))
    ));

    let replacement = world.create_body(BodyDesc::dynamic())?;
    assert_ne!(replacement.raw(), parent.raw());
    Ok(())
}

#[test]
fn update_fixed_clamps_substeps_and_exposes_interpolation_alpha() {
    let mut config = PhysicsConfig::default();
    config.timestep.fixed_dt = 0.1;
    config.timestep.max_frame_dt = 1.0;
    config.timestep.max_substeps = 2;
    let mut world = PhysicsWorld::new(config);

    let report = world.update_fixed(0.35);
    assert_eq!(report.steps_run, 2);
    assert_eq!(report.dropped_steps, 1);
    assert!((report.accumulator - 0.05).abs() < 0.0001);
    assert!((world.interpolation_alpha() - 0.5).abs() < 0.0001);
}

#[test]
fn queries_respect_filters_sensors_and_exclusions() -> PhysicsResult<()> {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());
    let (_, ground_collider) = add_ground(&mut world)?;
    let sensor = world.create_collider(
        ColliderDesc::sphere(1.0)
            .with_local_transform(Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)))
            .with_sensor(true),
    )?;

    let ray = Ray {
        origin: Vec3::new(0.0, 5.0, 0.0),
        direction: Vec3::new(0.0, -1.0, 0.0),
        max_toi: 10.0,
    };
    assert_eq!(
        world
            .query()
            .cast_ray(ray, QueryFilter::default())
            .unwrap()
            .collider,
        ground_collider
    );
    let mut include_sensor = QueryFilter::default();
    include_sensor.include_sensors = true;
    assert_eq!(
        world
            .query()
            .cast_ray(ray, include_sensor)
            .unwrap()
            .collider,
        sensor
    );

    let mut hits = Vec::new();
    let count = world.query().overlap_shape(
        OverlapInput {
            shape: ColliderShape::Sphere { radius: 0.25 },
            transform: Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
        },
        include_sensor,
        &mut hits,
    );
    assert_eq!(count, 1);
    assert_eq!(hits[0].collider, sensor);

    let mut point_hits = Vec::new();
    assert_eq!(
        world.query().contains_point(
            Vec3::new(0.0, 0.0, 0.0),
            QueryFilter::default(),
            &mut point_hits
        ),
        1
    );
    assert!(world
        .query()
        .project_point(Vec3::new(8.0, 0.0, 0.0), 20.0, true, QueryFilter::default())
        .is_some());
    Ok(())
}

#[test]
fn mesh_lifecycle_validates_missing_destroyed_and_in_use_resources() -> PhysicsResult<()> {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());
    let mesh = world.create_convex_mesh(ConvexMeshDesc {
        points: vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ],
    })?;
    let collider = world.create_collider(ColliderDesc::convex_hull(mesh))?;
    assert!(world.destroy_mesh(mesh).is_err());
    world.destroy_collider(collider)?;
    world.destroy_mesh(mesh)?;
    assert!(matches!(
        world.create_collider(ColliderDesc::convex_hull(mesh)),
        Err(PhysicsError::MeshNotFound(_))
    ));
    Ok(())
}

#[test]
fn command_buffer_reports_success_and_failure() -> PhysicsResult<()> {
    let mut world = PhysicsWorld::new(PhysicsConfig::default());
    let body = world.create_body(BodyDesc::dynamic())?;
    let mut commands = PhysicsCommandBuffer::new();
    commands.push(PhysicsCommand::SetBodyVelocity {
        body,
        velocity: Velocity {
            linear: Vec3::X,
            angular: Vec3::ZERO,
        },
        wake_up: true,
    });
    commands.push(PhysicsCommand::DestroyCollider(ColliderId::INVALID));

    let report = world.apply_commands(&mut commands);
    assert_eq!(report.applied, 1);
    assert_eq!(report.failed, 1);
    assert_eq!(world.body_velocity(body)?.linear, Vec3::X);
    Ok(())
}

#[test]
fn torque_and_angular_velocity_update_rotation() -> PhysicsResult<()> {
    let mut config = PhysicsConfig::default();
    config.gravity = Vec3::ZERO;
    let mut world = PhysicsWorld::new(config);
    let body = world.create_body(BodyDesc::dynamic())?;
    world.add_torque(body, Vec3::Y, ForceMode::VelocityChange, true)?;
    world.step_fixed(0.5);

    let rotation = world.body_transform(body)?.rotation;
    assert_ne!(rotation, Quat::IDENTITY);
    Ok(())
}

#[test]
fn character_snapshot_ecs_and_debug_paths_are_real() -> PhysicsResult<()> {
    let mut world = world_no_sleep();
    add_ground(&mut world)?;
    let body = world
        .create_body(BodyDesc::kinematic_position().with_translation(Vec3::new(0.0, 1.0, 0.0)))?;
    let collider =
        world.create_collider_with_parent(body, ColliderDesc::capsule(Axis3::Y, 0.5, 0.25))?;
    let controller = world.create_character_controller(CharacterControllerDesc::default());

    let output = world.move_character(CharacterMoveInput {
        controller,
        body,
        collider,
        desired_translation: Vec3::new(0.25, 0.0, 0.0),
        dt: 1.0 / 60.0,
        filter: QueryFilter::default(),
    })?;
    assert!(output.final_transform.translation.x > 0.0);

    let snapshot = world.snapshot();
    world.set_body_transform(
        body,
        Transform::from_translation(Vec3::new(5.0, 5.0, 0.0)),
        true,
    )?;
    world.restore(snapshot)?;
    assert!(world.body_transform(body)?.translation.x < 1.0);

    let mut entries = [(
        body,
        Transform::IDENTITY,
        PhysicsSyncComponent {
            mode: PhysicsSyncMode::PhysicsToTransform,
            interpolate: false,
        },
    )];
    let sync = world.sync_transforms_from_physics(&mut entries);
    assert_eq!(sync.applied, 1);

    let mut debug = DebugCollector::default();
    world.debug_draw(&mut debug, PhysicsDebugDrawOptions::default());
    assert!(!debug.spheres.is_empty() || !debug.capsules.is_empty());
    Ok(())
}

#[test]
fn character_dynamic_interaction_applies_velocity_change() -> PhysicsResult<()> {
    let mut config = PhysicsConfig::default();
    config.gravity = Vec3::ZERO;
    let mut world = PhysicsWorld::new(config);
    let obstacle =
        world.create_body(BodyDesc::dynamic().with_translation(Vec3::new(0.75, 0.0, 0.0)))?;
    world.create_collider_with_parent(obstacle, ColliderDesc::cuboid(Vec3::splat(0.25)))?;
    let character = world.create_body(BodyDesc::kinematic_position())?;
    let character_collider =
        world.create_collider_with_parent(character, ColliderDesc::capsule(Axis3::Y, 0.5, 0.25))?;
    let controller = world.create_character_controller(CharacterControllerDesc {
        apply_impulses_to_dynamic_bodies: true,
        ..Default::default()
    });

    world.move_character(CharacterMoveInput {
        controller,
        body: character,
        collider: character_collider,
        desired_translation: Vec3::X,
        dt: 1.0,
        filter: QueryFilter::default(),
    })?;

    assert!(world.body_velocity(obstacle)?.linear.x > 0.0);
    Ok(())
}

#[test]
fn joints_constrain_distance_and_errors_are_visible() -> PhysicsResult<()> {
    let mut config = PhysicsConfig::default();
    config.gravity = Vec3::ZERO;
    let mut world = PhysicsWorld::new(config);
    let fixed = world.create_body(BodyDesc::fixed())?;
    let dynamic =
        world.create_body(BodyDesc::dynamic().with_translation(Vec3::new(3.0, 0.0, 0.0)))?;
    let joint = world.create_joint(fixed, dynamic, JointDesc::distance(0.0, 1.0))?;

    world.step_fixed(1.0 / 60.0);
    assert!(world.body_transform(dynamic)?.translation.x <= 1.01);
    assert_eq!(world.joint_bodies(joint)?, (fixed, dynamic));
    assert!(matches!(
        world.create_joint(fixed, BodyId::INVALID, JointDesc::fixed()),
        Err(PhysicsError::BodyNotFound(_))
    ));
    Ok(())
}

#[test]
fn joint_motor_changes_dynamic_body_velocity() -> PhysicsResult<()> {
    let mut config = PhysicsConfig::default();
    config.gravity = Vec3::ZERO;
    let mut world = PhysicsWorld::new(config);
    let fixed = world.create_body(BodyDesc::fixed())?;
    let dynamic = world.create_body(BodyDesc::dynamic())?;
    let joint = world.create_joint(
        fixed,
        dynamic,
        JointDesc::Prismatic(PrismaticJointDesc {
            anchors: JointAnchor::default(),
            limits: None,
            motor: Some(JointMotor {
                target_velocity: 2.0,
                target_position: None,
                stiffness: 0.0,
                damping: 0.0,
                max_force: 10.0,
            }),
        }),
    )?;

    world.step_fixed(1.0 / 60.0);
    assert!(world.body_velocity(dynamic)?.linear.x > 0.0);
    assert!(world.contains_joint(joint));
    Ok(())
}

#[test]
fn hooks_can_disable_collision_pairs() -> PhysicsResult<()> {
    struct DisableAll;
    impl PhysicsHooks for DisableAll {
        fn filter_collision_pair(&self, _pair: CollisionPairInfo) -> CollisionDecision {
            CollisionDecision::DisableCollision
        }
    }

    let mut world = world_no_sleep();
    world.set_hooks(DisableAll);
    add_ground(&mut world)?;
    let body = world.create_body(BodyDesc::dynamic().with_translation(Vec3::new(0.0, 1.0, 0.0)))?;
    world.create_collider_with_parent(body, ColliderDesc::cuboid(Vec3::splat(0.5)))?;
    for _ in 0..60 {
        world.step_fixed(1.0 / 60.0);
    }
    assert!(world.body_transform(body)?.translation.y < 0.0);
    assert!(world.events().is_empty());
    Ok(())
}

#[test]
fn backend_capabilities_and_rapier_backend_run_real_simulation() -> PhysicsResult<()> {
    let mut backend = LocalPhysicsBackend::new(PhysicsConfig::default());
    assert!(backend.capabilities().rigid_bodies);
    let mut events = Vec::new();
    let report = backend.step(1.0 / 60.0, &mut events);
    assert_eq!(report.tick, PhysicsTick(1));

    #[cfg(feature = "backend_rapier")]
    {
        let mut config = PhysicsConfig::default();
        config.sleeping.enabled = false;

        let ground_desc = BodyDesc::fixed().with_translation(Vec3::new(0.0, -1.0, 0.0));
        let ground_collider_desc = ColliderDesc::cuboid(Vec3::new(5.0, 1.0, 5.0));
        let ball_desc = BodyDesc::dynamic().with_translation(Vec3::new(0.0, 2.0, 0.0));
        let ball_collider_desc = ColliderDesc::sphere(0.5);

        let mut ids = PhysicsWorld::new(config.clone());
        let ground = ids.create_body(ground_desc.clone())?;
        let ground_collider =
            ids.create_collider_with_parent(ground, ground_collider_desc.clone())?;
        let ball = ids.create_body(ball_desc.clone())?;
        let ball_collider = ids.create_collider_with_parent(ball, ball_collider_desc.clone())?;

        let mut rapier = rapier_backend::RapierPhysicsBackend::new(config.clone())?;
        assert_eq!(rapier.capabilities().backend_name, "rapier3d");
        assert!(rapier.capabilities().rigid_bodies);
        assert!(rapier.capabilities().raycast);

        rapier.create_body(ground, ground_desc)?;
        rapier.create_collider(ground_collider, Some(ground), ground_collider_desc)?;
        rapier.create_body(ball, ball_desc)?;
        rapier.create_collider(ball_collider, Some(ball), ball_collider_desc)?;

        let mut rapier_events = Vec::new();
        for _ in 0..120 {
            rapier.step(1.0 / 60.0, &mut rapier_events);
        }

        let snapshot = rapier.snapshot();
        let ball_snapshot = snapshot
            .bodies
            .iter()
            .find(|snapshot| snapshot.id == ball)
            .expect("ball snapshot");
        assert!(ball_snapshot.transform.translation.y >= 0.45);
        assert!(rapier_events.iter().any(|event| matches!(
            event,
            PhysicsEvent::CollisionStarted(collision)
                if (collision.a == ground_collider && collision.b == ball_collider)
                    || (collision.a == ball_collider && collision.b == ground_collider)
        )));

        let hit = rapier
            .query()
            .cast_ray(
                Ray {
                    origin: Vec3::new(3.0, 5.0, 0.0),
                    direction: Vec3::new(0.0, -1.0, 0.0),
                    max_toi: 10.0,
                },
                QueryFilter::default(),
            )
            .expect("ground ray hit");
        assert_eq!(hit.collider, ground_collider);

        let mut restored = rapier_backend::RapierPhysicsBackend::new(config)?;
        restored.restore(snapshot)?;
        assert_eq!(restored.snapshot().bodies.len(), 2);
    }

    Ok(())
}
