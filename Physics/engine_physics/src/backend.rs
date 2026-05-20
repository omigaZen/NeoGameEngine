use crate::body::BodyDesc;
use crate::collider::ColliderDesc;
use crate::config::{PhysicsConfig, PhysicsStepReport};
use crate::debug::{PhysicsDebugDrawOptions, PhysicsDebugRenderer};
use crate::error::{PhysicsError, PhysicsResult};
use crate::event::PhysicsEvent;
use crate::id::{BodyId, ColliderId, JointId};
use crate::joint::JointDesc;
use crate::math::Real;
use crate::query::{OverlapHit, OverlapInput, Ray, RayHit, ShapeCastHit, ShapeCastInput};
use crate::snapshot::PhysicsSnapshot;
use crate::world::{DestroyedObjects, PhysicsWorld};
use crate::{filter::QueryFilter, query::PhysicsQuery};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendCapabilities {
    pub backend_name: &'static str,
    pub rigid_bodies: bool,
    pub colliders: bool,
    pub fixed_step: bool,
    pub collision_events: bool,
    pub sensor_events: bool,
    pub contact_force_events: bool,
    pub raycast: bool,
    pub shapecast: bool,
    pub overlap: bool,
    pub character_controller: bool,
    pub joints: bool,
    pub debug_draw: bool,
    pub snapshot_restore: bool,
    pub hooks: bool,
    pub parallel: bool,
    pub deterministic: bool,
}

impl BackendCapabilities {
    pub fn local() -> Self {
        Self {
            backend_name: "engine_physics_local",
            rigid_bodies: true,
            colliders: true,
            fixed_step: true,
            collision_events: true,
            sensor_events: true,
            contact_force_events: true,
            raycast: true,
            shapecast: true,
            overlap: true,
            character_controller: true,
            joints: true,
            debug_draw: true,
            snapshot_restore: true,
            hooks: true,
            parallel: false,
            deterministic: true,
        }
    }

    pub fn rapier() -> Self {
        Self {
            backend_name: "rapier3d",
            rigid_bodies: true,
            colliders: true,
            fixed_step: true,
            collision_events: true,
            sensor_events: true,
            contact_force_events: true,
            raycast: true,
            shapecast: true,
            overlap: true,
            character_controller: false,
            joints: true,
            debug_draw: true,
            snapshot_restore: true,
            hooks: false,
            parallel: cfg!(feature = "parallel"),
            deterministic: cfg!(feature = "deterministic"),
        }
    }
}

pub trait PhysicsBackend {
    fn capabilities(&self) -> BackendCapabilities;
    fn create_body(&mut self, id: BodyId, desc: BodyDesc) -> PhysicsResult<()>;
    fn destroy_body(&mut self, id: BodyId, recursive: bool) -> PhysicsResult<DestroyedObjects>;

    fn create_collider(
        &mut self,
        id: ColliderId,
        parent: Option<BodyId>,
        desc: ColliderDesc,
    ) -> PhysicsResult<()>;

    fn destroy_collider(&mut self, id: ColliderId) -> PhysicsResult<()>;

    fn create_joint(
        &mut self,
        id: JointId,
        body_a: BodyId,
        body_b: BodyId,
        desc: JointDesc,
    ) -> PhysicsResult<()>;

    fn destroy_joint(&mut self, id: JointId) -> PhysicsResult<()>;

    fn step(&mut self, dt: Real, events: &mut Vec<PhysicsEvent>) -> PhysicsStepReport;
    fn query(&self) -> Box<dyn PhysicsQueryBackend + '_>;

    fn debug_draw(&self, renderer: &mut dyn PhysicsDebugRenderer, options: PhysicsDebugDrawOptions);

    fn snapshot(&self) -> PhysicsSnapshot;
    fn restore(&mut self, snapshot: PhysicsSnapshot) -> PhysicsResult<()>;
}

pub trait PhysicsQueryBackend {
    fn cast_ray(&self, ray: Ray, filter: QueryFilter) -> Option<RayHit>;

    fn cast_shape(&self, input: ShapeCastInput, filter: QueryFilter) -> Option<ShapeCastHit>;

    fn overlap_shape(
        &self,
        input: OverlapInput,
        filter: QueryFilter,
        hits: &mut Vec<OverlapHit>,
    ) -> usize;
}

pub struct LocalPhysicsBackend {
    world: PhysicsWorld,
}

impl LocalPhysicsBackend {
    pub fn new(config: PhysicsConfig) -> Self {
        Self {
            world: PhysicsWorld::new(config),
        }
    }

    pub fn try_new(config: PhysicsConfig) -> PhysicsResult<Self> {
        Ok(Self::new(config))
    }

    pub fn world(&self) -> &PhysicsWorld {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut PhysicsWorld {
        &mut self.world
    }
}

impl PhysicsBackend for LocalPhysicsBackend {
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::local()
    }

    fn create_body(&mut self, id: BodyId, desc: BodyDesc) -> PhysicsResult<()> {
        let actual = self.world.create_body(desc)?;
        if actual == id {
            Ok(())
        } else {
            Err(PhysicsError::Backend(format!(
                "local backend allocated {:?} instead of requested {:?}",
                actual, id
            )))
        }
    }

    fn destroy_body(&mut self, id: BodyId, recursive: bool) -> PhysicsResult<DestroyedObjects> {
        if recursive {
            self.world.destroy_body_recursive(id)
        } else {
            self.world.destroy_body(id).map(|_| DestroyedObjects {
                bodies: vec![id],
                ..Default::default()
            })
        }
    }

    fn create_collider(
        &mut self,
        id: ColliderId,
        parent: Option<BodyId>,
        desc: ColliderDesc,
    ) -> PhysicsResult<()> {
        let actual = if let Some(parent) = parent {
            self.world.create_collider_with_parent(parent, desc)?
        } else {
            self.world.create_collider(desc)?
        };
        if actual == id {
            Ok(())
        } else {
            Err(PhysicsError::Backend(format!(
                "local backend allocated {:?} instead of requested {:?}",
                actual, id
            )))
        }
    }

    fn destroy_collider(&mut self, id: ColliderId) -> PhysicsResult<()> {
        self.world.destroy_collider(id)
    }

    fn create_joint(
        &mut self,
        id: JointId,
        body_a: BodyId,
        body_b: BodyId,
        desc: JointDesc,
    ) -> PhysicsResult<()> {
        let actual = self.world.create_joint(body_a, body_b, desc)?;
        if actual == id {
            Ok(())
        } else {
            Err(PhysicsError::Backend(format!(
                "local backend allocated {:?} instead of requested {:?}",
                actual, id
            )))
        }
    }

    fn destroy_joint(&mut self, id: JointId) -> PhysicsResult<()> {
        self.world.destroy_joint(id)
    }

    fn step(&mut self, dt: Real, events: &mut Vec<PhysicsEvent>) -> PhysicsStepReport {
        let report = self.world.step_fixed(dt);
        events.extend(self.world.drain_events());
        report
    }

    fn query(&self) -> Box<dyn PhysicsQueryBackend + '_> {
        Box::new(LocalQueryBackend {
            query: self.world.query(),
        })
    }

    fn debug_draw(
        &self,
        renderer: &mut dyn PhysicsDebugRenderer,
        options: PhysicsDebugDrawOptions,
    ) {
        self.world.debug_draw(renderer, options);
    }

    fn snapshot(&self) -> PhysicsSnapshot {
        self.world.snapshot()
    }

    fn restore(&mut self, snapshot: PhysicsSnapshot) -> PhysicsResult<()> {
        self.world.restore(snapshot)
    }
}

struct LocalQueryBackend<'a> {
    query: PhysicsQuery<'a>,
}

impl PhysicsQueryBackend for LocalQueryBackend<'_> {
    fn cast_ray(&self, ray: Ray, filter: QueryFilter) -> Option<RayHit> {
        self.query.cast_ray(ray, filter)
    }

    fn cast_shape(&self, input: ShapeCastInput, filter: QueryFilter) -> Option<ShapeCastHit> {
        self.query.cast_shape(input, filter)
    }

    fn overlap_shape(
        &self,
        input: OverlapInput,
        filter: QueryFilter,
        hits: &mut Vec<OverlapHit>,
    ) -> usize {
        self.query.overlap_shape(input, filter, hits)
    }
}

#[cfg(feature = "backend_rapier")]
pub mod rapier_backend {
    use super::{BackendCapabilities, PhysicsBackend, PhysicsQueryBackend};
    use std::collections::BTreeMap;
    use std::sync::mpsc;

    use crate::body::{BodyDesc, BodyKind, LockedAxes, MassDesc, Velocity};
    use crate::collider::{
        ActiveEvents as EngineActiveEvents, Axis3, ColliderDesc, ColliderShape, TriMeshFlags,
    };
    use crate::config::{PhysicsConfig, PhysicsStepReport};
    use crate::debug::{
        DebugDrawCategory, DebugShapeStyle, PhysicsDebugDrawOptions, PhysicsDebugRenderer,
    };
    use crate::error::{PhysicsError, PhysicsResult};
    use crate::event::{
        CollisionEvent as EngineCollisionEvent, ContactForceEvent as EngineContactForceEvent,
        EventDropped, PhysicsEvent, SensorEvent,
    };
    use crate::filter::{CollisionFilter, QueryFilter};
    use crate::id::{BodyId, ColliderId, JointId, PhysicsMeshId, PhysicsTick};
    use crate::joint::{
        JointAxis as EngineJointAxis, JointDesc, JointLockedAxes, JointMotor as EngineJointMotor,
    };
    use crate::material::CombineRule;
    use crate::math::{Quat, Real, Transform, Vec3};
    use crate::mesh::PhysicsMeshDesc;
    use crate::query::{OverlapHit, OverlapInput, Ray, RayHit, ShapeCastHit, ShapeCastInput};
    use crate::snapshot::{
        BodySnapshot, ColliderSnapshot, JointSnapshot, PhysicsMeshSnapshot, PhysicsSnapshot,
    };
    use crate::world::DestroyedObjects;

    use rapier3d::geometry::{
        CollisionEvent as RapierCollisionEvent, CollisionEventFlags,
        ContactForceEvent as RapierContactForceEvent,
    };
    use rapier3d::parry::query::ShapeCastOptions;
    use rapier3d::pipeline::{ActiveEvents as RapierActiveEvents, ChannelEventCollector};
    use rapier3d::prelude::{
        BroadPhaseBvh, CCDSolver, CoefficientCombineRule, ColliderBuilder, ColliderHandle,
        ColliderSet, FixedJointBuilder, GenericJoint, GenericJointBuilder, Group,
        ImpulseJointHandle, ImpulseJointSet, IntegrationParameters, InteractionGroups,
        InteractionTestMode, IslandManager, JointAxesMask, JointAxis as RapierJointAxis,
        LockedAxes as RapierLockedAxes, MultibodyJointSet, NarrowPhase, PhysicsPipeline, Pose,
        PrismaticJointBuilder, QueryFilter as RapierQueryFilter, QueryFilterFlags,
        Ray as RapierRay, RevoluteJointBuilder, RigidBodyBuilder, RigidBodyHandle, RigidBodySet,
        RigidBodyType, RopeJointBuilder, Rotation, SharedShape, SphericalJointBuilder, Vector,
    };

    pub struct RapierPhysicsBackend {
        pipeline: PhysicsPipeline,
        integration: IntegrationParameters,
        islands: IslandManager,
        broad_phase: BroadPhaseBvh,
        narrow_phase: NarrowPhase,
        bodies: RigidBodySet,
        colliders: ColliderSet,
        impulse_joints: ImpulseJointSet,
        multibody_joints: MultibodyJointSet,
        ccd_solver: CCDSolver,
        config: PhysicsConfig,
        tick: PhysicsTick,
        frame_index: u64,
        accumulator: Real,
        body_handles: BTreeMap<BodyId, RigidBodyHandle>,
        body_descs: BTreeMap<BodyId, BodyDesc>,
        body_previous: BTreeMap<BodyId, Transform>,
        collider_handles: BTreeMap<ColliderId, ColliderHandle>,
        collider_descs: BTreeMap<ColliderId, ColliderDesc>,
        collider_parents: BTreeMap<ColliderId, Option<BodyId>>,
        joint_handles: BTreeMap<JointId, ImpulseJointHandle>,
        joint_snapshots: BTreeMap<JointId, JointSnapshot>,
        meshes: BTreeMap<PhysicsMeshId, PhysicsMeshDesc>,
    }

    impl RapierPhysicsBackend {
        pub fn new(config: PhysicsConfig) -> PhysicsResult<Self> {
            Ok(Self {
                pipeline: PhysicsPipeline::new(),
                integration: integration_from_config(&config),
                islands: IslandManager::new(),
                broad_phase: BroadPhaseBvh::new(),
                narrow_phase: NarrowPhase::new(),
                bodies: RigidBodySet::new(),
                colliders: ColliderSet::new(),
                impulse_joints: ImpulseJointSet::new(),
                multibody_joints: MultibodyJointSet::new(),
                ccd_solver: CCDSolver::new(),
                config,
                tick: PhysicsTick(0),
                frame_index: 0,
                accumulator: 0.0,
                body_handles: BTreeMap::new(),
                body_descs: BTreeMap::new(),
                body_previous: BTreeMap::new(),
                collider_handles: BTreeMap::new(),
                collider_descs: BTreeMap::new(),
                collider_parents: BTreeMap::new(),
                joint_handles: BTreeMap::new(),
                joint_snapshots: BTreeMap::new(),
                meshes: BTreeMap::new(),
            })
        }

        pub fn try_new(config: PhysicsConfig) -> PhysicsResult<Self> {
            Self::new(config)
        }

        fn body_handle(&self, id: BodyId) -> PhysicsResult<RigidBodyHandle> {
            self.body_handles
                .get(&id)
                .copied()
                .ok_or(PhysicsError::BodyNotFound(id))
        }

        fn collider_handle(&self, id: ColliderId) -> PhysicsResult<ColliderHandle> {
            self.collider_handles
                .get(&id)
                .copied()
                .ok_or(PhysicsError::ColliderNotFound(id))
        }

        fn body_id_from_handle(&self, handle: RigidBodyHandle) -> Option<BodyId> {
            self.body_handles
                .iter()
                .find_map(|(id, candidate)| (*candidate == handle).then_some(*id))
        }

        fn collider_id_from_handle(&self, handle: ColliderHandle) -> Option<ColliderId> {
            self.collider_handles
                .iter()
                .find_map(|(id, candidate)| (*candidate == handle).then_some(*id))
        }

        fn body_for_collider_handle(&self, handle: ColliderHandle) -> Option<BodyId> {
            self.colliders
                .get(handle)
                .and_then(|collider| collider.parent())
                .and_then(|body| self.body_id_from_handle(body))
        }

        fn collider_user_data(&self, collider: ColliderId) -> crate::id::PhysicsUserData {
            self.collider_descs
                .get(&collider)
                .map(|desc| desc.user_data)
                .unwrap_or_default()
        }

        fn drain_collision_event(&self, event: RapierCollisionEvent, out: &mut Vec<PhysicsEvent>) {
            let (a_handle, b_handle, flags, started) = match event {
                RapierCollisionEvent::Started(a, b, flags) => (a, b, flags, true),
                RapierCollisionEvent::Stopped(a, b, flags) => (a, b, flags, false),
            };
            let Some(a) = self.collider_id_from_handle(a_handle) else {
                return;
            };
            let Some(b) = self.collider_id_from_handle(b_handle) else {
                return;
            };
            let body_a = self.body_for_collider_handle(a_handle);
            let body_b = self.body_for_collider_handle(b_handle);
            if flags.contains(CollisionEventFlags::SENSOR) {
                let (sensor, other, sensor_body, other_body) =
                    if self.collider_descs.get(&a).is_some_and(|desc| desc.sensor) {
                        (a, b, body_a, body_b)
                    } else {
                        (b, a, body_b, body_a)
                    };
                let event = SensorEvent {
                    tick: self.tick,
                    sensor,
                    other,
                    sensor_body,
                    other_body,
                };
                out.push(if started {
                    PhysicsEvent::SensorEntered(event)
                } else {
                    PhysicsEvent::SensorExited(event)
                });
            } else {
                let event = EngineCollisionEvent {
                    tick: self.tick,
                    a,
                    b,
                    body_a,
                    body_b,
                };
                out.push(if started {
                    PhysicsEvent::CollisionStarted(event)
                } else {
                    PhysicsEvent::CollisionStopped(event)
                });
            }
        }

        fn drain_contact_force_event(
            &self,
            event: RapierContactForceEvent,
            out: &mut Vec<PhysicsEvent>,
        ) {
            let Some(a) = self.collider_id_from_handle(event.collider1) else {
                return;
            };
            let Some(b) = self.collider_id_from_handle(event.collider2) else {
                return;
            };
            out.push(PhysicsEvent::ContactForce(EngineContactForceEvent {
                tick: self.tick,
                a,
                b,
                body_a: self.body_for_collider_handle(event.collider1),
                body_b: self.body_for_collider_handle(event.collider2),
                total_force: er_vec3(event.total_force),
                total_force_magnitude: event.total_force_magnitude,
            }));
        }
    }

    impl PhysicsBackend for RapierPhysicsBackend {
        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::rapier()
        }

        fn create_body(&mut self, id: BodyId, desc: BodyDesc) -> PhysicsResult<()> {
            if self.body_handles.contains_key(&id) {
                return Err(PhysicsError::AlreadyExists);
            }
            validate_transform(desc.transform)?;
            let builder = body_builder(id, &desc);
            let handle = self.bodies.insert(builder);
            self.body_previous.insert(id, desc.transform);
            self.body_descs.insert(id, desc);
            self.body_handles.insert(id, handle);
            Ok(())
        }

        fn destroy_body(
            &mut self,
            id: BodyId,
            _recursive: bool,
        ) -> PhysicsResult<DestroyedObjects> {
            let handle = self.body_handle(id)?;
            let attached_colliders: Vec<_> = self
                .collider_handles
                .iter()
                .filter_map(|(collider_id, collider_handle)| {
                    self.colliders
                        .get(*collider_handle)
                        .is_some_and(|collider| collider.parent() == Some(handle))
                        .then_some(*collider_id)
                })
                .collect();
            let attached_joints: Vec<_> = self
                .joint_snapshots
                .iter()
                .filter_map(|(joint_id, snapshot)| {
                    (snapshot.body_a == id || snapshot.body_b == id).then_some(*joint_id)
                })
                .collect();

            self.bodies
                .remove(
                    handle,
                    &mut self.islands,
                    &mut self.colliders,
                    &mut self.impulse_joints,
                    &mut self.multibody_joints,
                    true,
                )
                .ok_or(PhysicsError::BodyNotFound(id))?;

            self.body_handles.remove(&id);
            self.body_descs.remove(&id);
            self.body_previous.remove(&id);
            for collider in &attached_colliders {
                self.collider_handles.remove(collider);
                self.collider_descs.remove(collider);
                self.collider_parents.remove(collider);
            }
            for joint in &attached_joints {
                self.joint_handles.remove(joint);
                self.joint_snapshots.remove(joint);
            }

            Ok(DestroyedObjects {
                bodies: vec![id],
                colliders: attached_colliders,
                joints: attached_joints,
                ..Default::default()
            })
        }

        fn create_collider(
            &mut self,
            id: ColliderId,
            parent: Option<BodyId>,
            desc: ColliderDesc,
        ) -> PhysicsResult<()> {
            if self.collider_handles.contains_key(&id) {
                return Err(PhysicsError::AlreadyExists);
            }
            validate_transform(desc.local_transform)?;
            let builder = collider_builder(id, &desc, &self.meshes)?;
            let handle = if let Some(parent) = parent {
                let body_handle = self.body_handle(parent)?;
                self.colliders
                    .insert_with_parent(builder, body_handle, &mut self.bodies)
            } else {
                self.colliders.insert(builder)
            };
            self.collider_handles.insert(id, handle);
            self.collider_descs.insert(id, desc);
            self.collider_parents.insert(id, parent);
            Ok(())
        }

        fn destroy_collider(&mut self, id: ColliderId) -> PhysicsResult<()> {
            let handle = self.collider_handle(id)?;
            self.colliders
                .remove(handle, &mut self.islands, &mut self.bodies, true)
                .ok_or(PhysicsError::ColliderNotFound(id))?;
            self.collider_handles.remove(&id);
            self.collider_descs.remove(&id);
            self.collider_parents.remove(&id);
            Ok(())
        }

        fn create_joint(
            &mut self,
            id: JointId,
            body_a: BodyId,
            body_b: BodyId,
            desc: JointDesc,
        ) -> PhysicsResult<()> {
            if self.joint_handles.contains_key(&id) {
                return Err(PhysicsError::AlreadyExists);
            }
            let handle_a = self.body_handle(body_a)?;
            let handle_b = self.body_handle(body_b)?;
            let joint = joint_from_desc(id, &desc)?;
            let handle = self.impulse_joints.insert(handle_a, handle_b, joint, true);
            self.joint_handles.insert(id, handle);
            self.joint_snapshots.insert(
                id,
                JointSnapshot {
                    id,
                    body_a,
                    body_b,
                    desc,
                    enabled: true,
                },
            );
            Ok(())
        }

        fn destroy_joint(&mut self, id: JointId) -> PhysicsResult<()> {
            let handle = self
                .joint_handles
                .get(&id)
                .copied()
                .ok_or(PhysicsError::JointNotFound(id))?;
            self.impulse_joints
                .remove(handle, true)
                .ok_or(PhysicsError::JointNotFound(id))?;
            self.joint_handles.remove(&id);
            self.joint_snapshots.remove(&id);
            Ok(())
        }

        fn step(&mut self, dt: Real, events: &mut Vec<PhysicsEvent>) -> PhysicsStepReport {
            let dt = if dt.is_finite() && dt > 0.0 {
                dt
            } else {
                self.config.timestep.fixed_dt
            };
            self.tick.0 += 1;
            self.integration.dt = dt;
            self.body_previous = self
                .body_handles
                .iter()
                .filter_map(|(id, handle)| {
                    self.bodies
                        .get(*handle)
                        .map(|body| (*id, transform_from_pose(body.position())))
                })
                .collect();

            let (collision_tx, collision_rx) = mpsc::channel();
            let (contact_tx, contact_rx) = mpsc::channel();
            let event_handler = ChannelEventCollector::new(collision_tx, contact_tx);
            self.pipeline.step(
                rvec3(self.config.gravity),
                &self.integration,
                &mut self.islands,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.bodies,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                &mut self.ccd_solver,
                &(),
                &event_handler,
            );

            let mut generated = Vec::new();
            for event in collision_rx.try_iter() {
                self.drain_collision_event(event, &mut generated);
            }
            for event in contact_rx.try_iter() {
                self.drain_contact_force_event(event, &mut generated);
            }

            let events_generated = generated.len();
            let max_events = self.config.events.max_events_per_tick;
            if max_events == 0 {
                if events_generated > 0 {
                    events.push(PhysicsEvent::EventDropped(EventDropped {
                        tick: self.tick,
                        dropped: events_generated,
                        max_events_per_tick: max_events,
                    }));
                }
            } else {
                let dropped = generated.len().saturating_sub(max_events);
                events.extend(generated.into_iter().take(max_events));
                if dropped > 0 {
                    events.push(PhysicsEvent::EventDropped(EventDropped {
                        tick: self.tick,
                        dropped,
                        max_events_per_tick: max_events,
                    }));
                }
            }

            let active_bodies = self
                .bodies
                .iter()
                .filter(|(_, body)| {
                    body.is_enabled()
                        && !body.is_sleeping()
                        && !matches!(body.body_type(), RigidBodyType::Fixed)
                })
                .count();

            PhysicsStepReport {
                tick: self.tick,
                dt,
                active_bodies,
                events_generated,
                commands_applied: 0,
            }
        }

        fn query(&self) -> Box<dyn PhysicsQueryBackend + '_> {
            Box::new(RapierQueryBackend { backend: self })
        }

        fn debug_draw(
            &self,
            renderer: &mut dyn PhysicsDebugRenderer,
            options: PhysicsDebugDrawOptions,
        ) {
            if !options.draw_colliders {
                return;
            }
            for (id, desc) in &self.collider_descs {
                let Some(handle) = self.collider_handles.get(id) else {
                    continue;
                };
                let Some(collider) = self.colliders.get(*handle) else {
                    continue;
                };
                let category = debug_category(
                    desc,
                    collider.parent().and_then(|handle| self.bodies.get(handle)),
                );
                let transform = transform_from_pose(collider.position());
                draw_shape(renderer, &desc.shape, transform, category);
                if options.draw_names {
                    if let Some(name) = &desc.debug_name {
                        renderer.text(transform.translation, name);
                    }
                }
            }
        }

        fn snapshot(&self) -> PhysicsSnapshot {
            let bodies = self
                .body_descs
                .iter()
                .filter_map(|(id, desc)| {
                    let body = self.bodies.get(*self.body_handles.get(id)?)?;
                    let transform = transform_from_pose(body.position());
                    let velocity = Velocity {
                        linear: er_vec3(body.linvel()),
                        angular: er_vec3(body.angvel()),
                    };
                    Some(BodySnapshot {
                        id: *id,
                        desc: desc.clone(),
                        transform,
                        previous_transform: self
                            .body_previous
                            .get(id)
                            .copied()
                            .unwrap_or(transform),
                        velocity,
                        sleeping: body.is_sleeping(),
                    })
                })
                .collect();
            let colliders = self
                .collider_descs
                .iter()
                .map(|(id, desc)| ColliderSnapshot {
                    id: *id,
                    parent: self.collider_parents.get(id).copied().flatten(),
                    desc: desc.clone(),
                })
                .collect();
            let meshes = self
                .meshes
                .iter()
                .map(|(id, desc)| PhysicsMeshSnapshot {
                    id: *id,
                    desc: desc.clone(),
                })
                .collect();
            PhysicsSnapshot {
                tick: self.tick,
                frame_index: self.frame_index,
                accumulator: self.accumulator,
                config: self.config.clone(),
                bodies,
                colliders,
                joints: self.joint_snapshots.values().cloned().collect(),
                meshes,
                character_controllers: Vec::new(),
            }
        }

        fn restore(&mut self, snapshot: PhysicsSnapshot) -> PhysicsResult<()> {
            *self = Self::new(snapshot.config.clone())?;
            self.tick = snapshot.tick;
            self.frame_index = snapshot.frame_index;
            self.accumulator = snapshot.accumulator;
            for mesh in snapshot.meshes {
                self.meshes.insert(mesh.id, mesh.desc);
            }
            let mut bodies = snapshot.bodies;
            bodies.sort_by_key(|body| body.id.raw());
            for body in bodies {
                let mut desc = body.desc;
                desc.transform = body.transform;
                desc.velocity = body.velocity;
                self.create_body(body.id, desc)?;
                self.body_previous.insert(body.id, body.previous_transform);
                if body.sleeping {
                    if let Some(handle) = self.body_handles.get(&body.id).copied() {
                        if let Some(rapier_body) = self.bodies.get_mut(handle) {
                            rapier_body.sleep();
                        }
                    }
                }
            }
            let mut colliders = snapshot.colliders;
            colliders.sort_by_key(|collider| collider.id.raw());
            for collider in colliders {
                self.create_collider(collider.id, collider.parent, collider.desc)?;
            }
            let mut joints = snapshot.joints;
            joints.sort_by_key(|joint| joint.id.raw());
            for joint in joints {
                self.create_joint(joint.id, joint.body_a, joint.body_b, joint.desc.clone())?;
                if !joint.enabled {
                    if let Some(handle) = self.joint_handles.get(&joint.id).copied() {
                        if let Some(rapier_joint) = self.impulse_joints.get_mut(handle, false) {
                            rapier_joint.data.set_enabled(false);
                        }
                    }
                    if let Some(snapshot) = self.joint_snapshots.get_mut(&joint.id) {
                        snapshot.enabled = false;
                    }
                }
            }
            Ok(())
        }
    }

    struct RapierQueryBackend<'a> {
        backend: &'a RapierPhysicsBackend,
    }

    impl PhysicsQueryBackend for RapierQueryBackend<'_> {
        fn cast_ray(&self, ray: Ray, filter: QueryFilter) -> Option<RayHit> {
            let max_toi = ray.max_toi;
            let ray = RapierRay::new(rvec3(ray.origin), rvec3(ray.direction));
            let query_filter = self.to_query_filter(filter);
            let query = self.backend.broad_phase.as_query_pipeline(
                self.backend.narrow_phase.query_dispatcher(),
                &self.backend.bodies,
                &self.backend.colliders,
                query_filter,
            );
            let (handle, hit) = query.cast_ray_and_get_normal(&ray, max_toi, true)?;
            self.ray_hit(handle, &ray, hit)
        }

        fn cast_shape(&self, input: ShapeCastInput, filter: QueryFilter) -> Option<ShapeCastHit> {
            let shape = shared_shape_from_desc(&input.shape, &self.backend.meshes).ok()?;
            let query_filter = self.to_query_filter(filter);
            let query = self.backend.broad_phase.as_query_pipeline(
                self.backend.narrow_phase.query_dispatcher(),
                &self.backend.bodies,
                &self.backend.colliders,
                query_filter,
            );
            let options = ShapeCastOptions {
                max_time_of_impact: input.max_toi,
                target_distance: input.target_distance,
                stop_at_penetration: input.stop_at_penetration,
                compute_impact_geometry_on_penetration: true,
            };
            let (handle, hit) = query.cast_shape(
                &pose_from_transform(input.transform),
                rvec3(input.translation),
                shape.as_ref(),
                options,
            )?;
            let collider = self.backend.collider_id_from_handle(handle)?;
            Some(ShapeCastHit {
                collider,
                body: self.backend.body_for_collider_handle(handle),
                toi: hit.time_of_impact,
                point1: er_vec3(hit.witness1),
                point2: er_vec3(hit.witness2),
                normal1: er_vec3(hit.normal1),
                normal2: er_vec3(hit.normal2),
                user_data: self.backend.collider_user_data(collider),
            })
        }

        fn overlap_shape(
            &self,
            input: OverlapInput,
            filter: QueryFilter,
            hits: &mut Vec<OverlapHit>,
        ) -> usize {
            let Some(shape) = shared_shape_from_desc(&input.shape, &self.backend.meshes).ok()
            else {
                return 0;
            };
            let max_results = filter.max_results.unwrap_or(usize::MAX);
            let query_filter = self.to_query_filter(filter);
            let query = self.backend.broad_phase.as_query_pipeline(
                self.backend.narrow_phase.query_dispatcher(),
                &self.backend.bodies,
                &self.backend.colliders,
                query_filter,
            );
            let start = hits.len();
            for (handle, _) in
                query.intersect_shape(pose_from_transform(input.transform), shape.as_ref())
            {
                if hits.len() - start >= max_results {
                    break;
                }
                if let Some(collider) = self.backend.collider_id_from_handle(handle) {
                    hits.push(OverlapHit {
                        collider,
                        body: self.backend.body_for_collider_handle(handle),
                        user_data: self.backend.collider_user_data(collider),
                    });
                }
            }
            hits.len() - start
        }
    }

    impl RapierQueryBackend<'_> {
        fn to_query_filter(&self, filter: QueryFilter) -> RapierQueryFilter<'_> {
            let mut flags = QueryFilterFlags::empty();
            if !filter.include_sensors {
                flags |= QueryFilterFlags::EXCLUDE_SENSORS;
            }
            if !filter.include_dynamic {
                flags |= QueryFilterFlags::EXCLUDE_DYNAMIC;
            }
            if !filter.include_fixed {
                flags |= QueryFilterFlags::EXCLUDE_FIXED;
            }
            if !filter.include_kinematic {
                flags |= QueryFilterFlags::EXCLUDE_KINEMATIC;
            }
            RapierQueryFilter {
                flags,
                groups: Some(interaction_groups(filter.groups, filter.mask)),
                exclude_collider: filter
                    .exclude_collider
                    .and_then(|id| self.backend.collider_handles.get(&id).copied()),
                exclude_rigid_body: filter
                    .exclude_body
                    .and_then(|id| self.backend.body_handles.get(&id).copied()),
                predicate: None,
            }
        }

        fn ray_hit(
            &self,
            handle: ColliderHandle,
            ray: &RapierRay,
            hit: rapier3d::geometry::RayIntersection,
        ) -> Option<RayHit> {
            let collider = self.backend.collider_id_from_handle(handle)?;
            Some(RayHit {
                collider,
                body: self.backend.body_for_collider_handle(handle),
                point: er_vec3(ray.point_at(hit.time_of_impact)),
                normal: er_vec3(hit.normal),
                toi: hit.time_of_impact,
                user_data: self.backend.collider_user_data(collider),
            })
        }
    }

    fn integration_from_config(config: &PhysicsConfig) -> IntegrationParameters {
        let mut integration = IntegrationParameters::default();
        integration.dt = config.timestep.fixed_dt;
        integration.num_solver_iterations = config.solver.velocity_iterations.max(1) as usize;
        integration.num_internal_stabilization_iterations =
            config.solver.stabilization_iterations as usize;
        integration.normalized_allowed_linear_error = config.solver.allowed_linear_error;
        integration.normalized_prediction_distance = config.solver.prediction_distance;
        integration.max_ccd_substeps = if config.ccd.enabled {
            config.ccd.max_substeps.max(1) as usize
        } else {
            1
        };
        integration
    }

    fn body_builder(id: BodyId, desc: &BodyDesc) -> RigidBodyBuilder {
        let mut builder = match desc.kind {
            BodyKind::Dynamic => RigidBodyBuilder::dynamic(),
            BodyKind::Fixed => RigidBodyBuilder::fixed(),
            BodyKind::KinematicPosition => RigidBodyBuilder::kinematic_position_based(),
            BodyKind::KinematicVelocity => RigidBodyBuilder::kinematic_velocity_based(),
        }
        .pose(pose_from_transform(desc.transform))
        .linvel(rvec3(desc.velocity.linear))
        .angvel(rvec3(desc.velocity.angular))
        .linear_damping(desc.damping.linear)
        .angular_damping(desc.damping.angular)
        .gravity_scale(desc.gravity_scale)
        .locked_axes(locked_axes(desc.lock_axes))
        .ccd_enabled(desc.ccd_enabled)
        .can_sleep(desc.can_sleep)
        .enabled(desc.enabled)
        .user_data(id.raw() as u128);

        if let MassDesc::Explicit { mass, .. } = desc.mass {
            builder = builder.additional_mass(mass);
        }

        builder
    }

    fn collider_builder(
        id: ColliderId,
        desc: &ColliderDesc,
        meshes: &BTreeMap<PhysicsMeshId, PhysicsMeshDesc>,
    ) -> PhysicsResult<ColliderBuilder> {
        let mut builder = ColliderBuilder::new(shared_shape_from_desc(&desc.shape, meshes)?)
            .position(pose_from_transform(desc.local_transform))
            .sensor(desc.sensor)
            .enabled(desc.enabled)
            .density(desc.density)
            .friction(desc.material.friction)
            .friction_combine_rule(combine_rule(desc.material.friction_combine))
            .restitution(desc.material.restitution)
            .restitution_combine_rule(combine_rule(desc.material.restitution_combine))
            .collision_groups(collision_groups(desc.filter))
            .solver_groups(collision_groups(desc.filter))
            .contact_skin(desc.contact_skin)
            .user_data(id.raw() as u128);

        let active_events = active_events(desc.events);
        if !active_events.is_empty() {
            builder = builder.active_events(active_events);
        }
        if desc
            .events
            .contains(EngineActiveEvents::CONTACT_FORCE_EVENTS)
        {
            builder = builder.contact_force_event_threshold(0.0);
        }
        Ok(builder)
    }

    fn shared_shape_from_desc(
        shape: &ColliderShape,
        meshes: &BTreeMap<PhysicsMeshId, PhysicsMeshDesc>,
    ) -> PhysicsResult<SharedShape> {
        match shape {
            ColliderShape::Sphere { radius } => {
                validate_positive(*radius, "sphere radius").map(|_| SharedShape::ball(*radius))
            }
            ColliderShape::Cuboid { half_extents } => {
                validate_vec3_positive(*half_extents, "cuboid half extents")?;
                Ok(SharedShape::cuboid(
                    half_extents.x,
                    half_extents.y,
                    half_extents.z,
                ))
            }
            ColliderShape::Capsule {
                axis,
                half_height,
                radius,
            } => {
                validate_positive(*half_height, "capsule half height")?;
                validate_positive(*radius, "capsule radius")?;
                Ok(match axis {
                    Axis3::X => SharedShape::capsule_x(*half_height, *radius),
                    Axis3::Y => SharedShape::capsule_y(*half_height, *radius),
                    Axis3::Z => SharedShape::capsule_z(*half_height, *radius),
                })
            }
            ColliderShape::Cylinder {
                axis,
                half_height,
                radius,
            } => {
                validate_positive(*half_height, "cylinder half height")?;
                validate_positive(*radius, "cylinder radius")?;
                let shape = SharedShape::cylinder(*half_height, *radius);
                Ok(oriented_y_shape(*axis, shape))
            }
            ColliderShape::Cone {
                axis,
                half_height,
                radius,
            } => {
                validate_positive(*half_height, "cone half height")?;
                validate_positive(*radius, "cone radius")?;
                let shape = SharedShape::cone(*half_height, *radius);
                Ok(oriented_y_shape(*axis, shape))
            }
            ColliderShape::Compound { parts } => {
                if parts.is_empty() {
                    return Err(invalid_shape("compound shape requires at least one part"));
                }
                let mut rapier_parts = Vec::with_capacity(parts.len());
                for part in parts {
                    validate_transform(part.local_transform)?;
                    rapier_parts.push((
                        pose_from_transform(part.local_transform),
                        shared_shape_from_desc(&part.shape, meshes)?,
                    ));
                }
                Ok(SharedShape::compound(rapier_parts))
            }
            ColliderShape::ConvexHull { mesh } => match meshes.get(mesh) {
                Some(PhysicsMeshDesc::Convex(desc)) => SharedShape::convex_hull(
                    &desc.points.iter().copied().map(rvec3).collect::<Vec<_>>(),
                )
                .ok_or_else(|| invalid_shape("convex hull generation failed")),
                Some(other) => SharedShape::convex_hull(
                    &other.points().into_iter().map(rvec3).collect::<Vec<_>>(),
                )
                .ok_or_else(|| invalid_shape("convex hull generation failed")),
                None => Err(PhysicsError::MeshNotFound(*mesh)),
            },
            ColliderShape::TriMesh { mesh, flags } => match meshes.get(mesh) {
                Some(PhysicsMeshDesc::TriMesh(desc)) => {
                    let vertices = desc.vertices.iter().copied().map(rvec3).collect();
                    let indices = desc.indices.clone();
                    let builder_flags = trimesh_flags(*flags);
                    SharedShape::trimesh_with_flags(vertices, indices, builder_flags)
                        .map_err(|err| invalid_shape(&format!("trimesh generation failed: {err}")))
                }
                Some(_) => Err(invalid_shape(
                    "trimesh collider requires a trimesh resource",
                )),
                None => Err(PhysicsError::MeshNotFound(*mesh)),
            },
            ColliderShape::HeightField { mesh } => match meshes.get(mesh) {
                Some(PhysicsMeshDesc::HeightField(desc)) => {
                    let heights = rapier3d::parry::utils::Array2::new(
                        desc.rows as usize,
                        desc.cols as usize,
                        desc.heights.clone(),
                    );
                    Ok(SharedShape::heightfield(heights, rvec3(desc.scale)))
                }
                Some(_) => Err(invalid_shape(
                    "heightfield collider requires a heightfield resource",
                )),
                None => Err(PhysicsError::MeshNotFound(*mesh)),
            },
        }
    }

    fn joint_from_desc(id: JointId, desc: &JointDesc) -> PhysicsResult<GenericJoint> {
        let joint = match desc {
            JointDesc::Fixed(desc) => FixedJointBuilder::new()
                .local_frame1(pose_from_transform(desc.local_frame_a))
                .local_frame2(pose_from_transform(desc.local_frame_b))
                .build()
                .into(),
            JointDesc::Ball(desc) => {
                let mut builder = SphericalJointBuilder::new()
                    .local_anchor1(rvec3(desc.anchors.local_anchor_a))
                    .local_anchor2(rvec3(desc.anchors.local_anchor_b));
                if let Some(limits) = desc.limits {
                    builder = builder.limits(RapierJointAxis::AngX, [limits.min, limits.max]);
                }
                builder.build().into()
            }
            JointDesc::Hinge(desc) => {
                let mut builder = RevoluteJointBuilder::new(axis_or_x(desc.anchors.local_axis_a))
                    .local_anchor1(rvec3(desc.anchors.local_anchor_a))
                    .local_anchor2(rvec3(desc.anchors.local_anchor_b));
                if let Some(limits) = desc.limits {
                    builder = builder.limits([limits.min, limits.max]);
                }
                if let Some(motor) = desc.motor {
                    builder = apply_revolute_motor(builder, motor);
                }
                builder.build().into()
            }
            JointDesc::Prismatic(desc) => {
                let mut builder = PrismaticJointBuilder::new(axis_or_x(desc.anchors.local_axis_a))
                    .local_anchor1(rvec3(desc.anchors.local_anchor_a))
                    .local_anchor2(rvec3(desc.anchors.local_anchor_b));
                if let Some(limits) = desc.limits {
                    builder = builder.limits([limits.min, limits.max]);
                }
                if let Some(motor) = desc.motor {
                    builder = apply_prismatic_motor(builder, motor);
                }
                builder.build().into()
            }
            JointDesc::Distance(desc) => {
                RopeJointBuilder::new(desc.max_distance.max(Real::EPSILON))
                    .local_anchor1(rvec3(desc.local_anchor_a))
                    .local_anchor2(rvec3(desc.local_anchor_b))
                    .build()
                    .into()
            }
            JointDesc::Generic(desc) => {
                let mut builder = GenericJointBuilder::new(joint_axes_mask(desc.locked_axes))
                    .local_frame1(pose_from_transform(desc.local_frame_a))
                    .local_frame2(pose_from_transform(desc.local_frame_b));
                for limit in &desc.limits {
                    builder = builder.limits(joint_axis(limit.axis), [limit.min, limit.max]);
                }
                for motor in &desc.motors {
                    builder = apply_generic_motor(builder, motor.axis, motor.motor);
                }
                builder.build()
            }
        };
        Ok(joint.with_user_data(id.raw() as u128))
    }

    trait GenericJointUserData {
        fn with_user_data(self, user_data: u128) -> Self;
    }

    impl GenericJointUserData for GenericJoint {
        fn with_user_data(mut self, user_data: u128) -> Self {
            self.user_data = user_data;
            self
        }
    }

    fn apply_revolute_motor(
        mut builder: RevoluteJointBuilder,
        motor: EngineJointMotor,
    ) -> RevoluteJointBuilder {
        let target = motor.target_position.unwrap_or(0.0);
        builder = builder.motor(
            target,
            motor.target_velocity,
            motor.stiffness,
            motor.damping,
        );
        builder.motor_max_force(motor.max_force)
    }

    fn apply_prismatic_motor(
        mut builder: PrismaticJointBuilder,
        motor: EngineJointMotor,
    ) -> PrismaticJointBuilder {
        let target = motor.target_position.unwrap_or(0.0);
        builder = builder.set_motor(
            target,
            motor.target_velocity,
            motor.stiffness,
            motor.damping,
        );
        builder.motor_max_force(motor.max_force)
    }

    fn apply_generic_motor(
        mut builder: GenericJointBuilder,
        axis: EngineJointAxis,
        motor: EngineJointMotor,
    ) -> GenericJointBuilder {
        let target = motor.target_position.unwrap_or(0.0);
        builder = builder.set_motor(
            joint_axis(axis),
            target,
            motor.target_velocity,
            motor.stiffness,
            motor.damping,
        );
        builder.motor_max_force(joint_axis(axis), motor.max_force)
    }

    fn active_events(events: EngineActiveEvents) -> RapierActiveEvents {
        let mut active = RapierActiveEvents::empty();
        if events
            .intersects(EngineActiveEvents::COLLISION_EVENTS | EngineActiveEvents::SENSOR_EVENTS)
        {
            active |= RapierActiveEvents::COLLISION_EVENTS;
        }
        if events.contains(EngineActiveEvents::CONTACT_FORCE_EVENTS) {
            active |= RapierActiveEvents::CONTACT_FORCE_EVENTS;
        }
        active
    }

    fn collision_groups(filter: CollisionFilter) -> InteractionGroups {
        interaction_groups(filter.groups, filter.mask)
    }

    fn interaction_groups(groups: u32, mask: u32) -> InteractionGroups {
        InteractionGroups::new(
            Group::from_bits_retain(groups),
            Group::from_bits_retain(mask),
            InteractionTestMode::And,
        )
    }

    fn locked_axes(axes: LockedAxes) -> RapierLockedAxes {
        let mut out = RapierLockedAxes::empty();
        out.set(
            RapierLockedAxes::TRANSLATION_LOCKED_X,
            axes.contains(LockedAxes::TRANSLATION_X),
        );
        out.set(
            RapierLockedAxes::TRANSLATION_LOCKED_Y,
            axes.contains(LockedAxes::TRANSLATION_Y),
        );
        out.set(
            RapierLockedAxes::TRANSLATION_LOCKED_Z,
            axes.contains(LockedAxes::TRANSLATION_Z),
        );
        out.set(
            RapierLockedAxes::ROTATION_LOCKED_X,
            axes.contains(LockedAxes::ROTATION_X),
        );
        out.set(
            RapierLockedAxes::ROTATION_LOCKED_Y,
            axes.contains(LockedAxes::ROTATION_Y),
        );
        out.set(
            RapierLockedAxes::ROTATION_LOCKED_Z,
            axes.contains(LockedAxes::ROTATION_Z),
        );
        out
    }

    fn joint_axes_mask(axes: JointLockedAxes) -> JointAxesMask {
        let mut out = JointAxesMask::empty();
        out.set(JointAxesMask::LIN_X, axes.contains(JointLockedAxes::LIN_X));
        out.set(JointAxesMask::LIN_Y, axes.contains(JointLockedAxes::LIN_Y));
        out.set(JointAxesMask::LIN_Z, axes.contains(JointLockedAxes::LIN_Z));
        out.set(JointAxesMask::ANG_X, axes.contains(JointLockedAxes::ANG_X));
        out.set(JointAxesMask::ANG_Y, axes.contains(JointLockedAxes::ANG_Y));
        out.set(JointAxesMask::ANG_Z, axes.contains(JointLockedAxes::ANG_Z));
        out
    }

    fn joint_axis(axis: EngineJointAxis) -> RapierJointAxis {
        match axis {
            EngineJointAxis::X => RapierJointAxis::LinX,
            EngineJointAxis::Y => RapierJointAxis::LinY,
            EngineJointAxis::Z => RapierJointAxis::LinZ,
            EngineJointAxis::AngularX => RapierJointAxis::AngX,
            EngineJointAxis::AngularY => RapierJointAxis::AngY,
            EngineJointAxis::AngularZ => RapierJointAxis::AngZ,
        }
    }

    fn combine_rule(rule: CombineRule) -> CoefficientCombineRule {
        match rule {
            CombineRule::Average => CoefficientCombineRule::Average,
            CombineRule::Min => CoefficientCombineRule::Min,
            CombineRule::Max => CoefficientCombineRule::Max,
            CombineRule::Multiply => CoefficientCombineRule::Multiply,
        }
    }

    fn trimesh_flags(flags: TriMeshFlags) -> rapier3d::parry::shape::TriMeshFlags {
        let mut out = rapier3d::parry::shape::TriMeshFlags::empty();
        out.set(
            rapier3d::parry::shape::TriMeshFlags::FIX_INTERNAL_EDGES,
            flags.contains(TriMeshFlags::FIX_INTERNAL_EDGES),
        );
        out
    }

    fn oriented_y_shape(axis: Axis3, shape: SharedShape) -> SharedShape {
        match axis {
            Axis3::Y => shape,
            Axis3::X => SharedShape::compound(vec![(
                Pose::from_parts(
                    Vector::ZERO,
                    Rotation::from_rotation_arc(Vector::Y, Vector::X),
                ),
                shape,
            )]),
            Axis3::Z => SharedShape::compound(vec![(
                Pose::from_parts(
                    Vector::ZERO,
                    Rotation::from_rotation_arc(Vector::Y, Vector::Z),
                ),
                shape,
            )]),
        }
    }

    fn pose_from_transform(transform: Transform) -> Pose {
        Pose::from_parts(rvec3(transform.translation), rquat(transform.rotation))
    }

    fn transform_from_pose(pose: &Pose) -> Transform {
        Transform {
            translation: er_vec3(pose.translation),
            rotation: equat(pose.rotation),
            scale: Vec3::ONE,
        }
    }

    fn rvec3(v: Vec3) -> Vector {
        Vector::new(v.x, v.y, v.z)
    }

    fn er_vec3(v: Vector) -> Vec3 {
        Vec3::new(v.x, v.y, v.z)
    }

    fn rquat(q: Quat) -> Rotation {
        Rotation::from_xyzw(q.x, q.y, q.z, q.w).normalize()
    }

    fn equat(q: Rotation) -> Quat {
        Quat::from_xyzw(q.x, q.y, q.z, q.w).normalized()
    }

    fn axis_or_x(axis: Vec3) -> Vector {
        let axis = axis.normalize_or_zero();
        if axis == Vec3::ZERO {
            Vector::X
        } else {
            rvec3(axis)
        }
    }

    fn debug_category(
        desc: &ColliderDesc,
        parent: Option<&rapier3d::prelude::RigidBody>,
    ) -> DebugDrawCategory {
        if desc.sensor {
            return DebugDrawCategory::Sensor;
        }
        if parent.is_some_and(|body| body.is_sleeping()) {
            return DebugDrawCategory::Sleeping;
        }
        match parent.map(|body| body.body_type()) {
            Some(RigidBodyType::Dynamic) => DebugDrawCategory::DynamicBody,
            Some(RigidBodyType::KinematicPositionBased)
            | Some(RigidBodyType::KinematicVelocityBased) => DebugDrawCategory::KinematicBody,
            _ => DebugDrawCategory::FixedBody,
        }
    }

    fn draw_shape(
        renderer: &mut dyn PhysicsDebugRenderer,
        shape: &ColliderShape,
        transform: Transform,
        category: DebugDrawCategory,
    ) {
        let style = DebugShapeStyle::new(category);
        match shape {
            ColliderShape::Sphere { radius } => {
                renderer.sphere(transform.translation, *radius, style);
            }
            ColliderShape::Cuboid { half_extents } => {
                renderer.cuboid(transform, *half_extents, style);
            }
            ColliderShape::Capsule {
                axis,
                half_height,
                radius,
            } => renderer.capsule(transform, *axis, *half_height, *radius, style),
            ColliderShape::Compound { parts } => {
                for part in parts {
                    draw_shape(
                        renderer,
                        &part.shape,
                        Transform::compose(transform, part.local_transform),
                        category,
                    );
                }
            }
            _ => {
                renderer.cuboid(transform, Vec3::splat(0.5), DebugShapeStyle::new(category));
            }
        }
    }

    fn validate_transform(transform: Transform) -> PhysicsResult<()> {
        if transform.is_finite() {
            Ok(())
        } else {
            Err(PhysicsError::InvalidTransform)
        }
    }

    fn validate_positive(value: Real, name: &str) -> PhysicsResult<()> {
        if value.is_finite() && value > 0.0 {
            Ok(())
        } else {
            Err(invalid_shape(&format!(
                "{name} must be positive and finite"
            )))
        }
    }

    fn validate_vec3_positive(value: Vec3, name: &str) -> PhysicsResult<()> {
        if value.is_finite() && value.x > 0.0 && value.y > 0.0 && value.z > 0.0 {
            Ok(())
        } else {
            Err(invalid_shape(&format!(
                "{name} must be positive and finite"
            )))
        }
    }

    fn invalid_shape(reason: &str) -> PhysicsError {
        PhysicsError::InvalidShape {
            reason: reason.to_owned(),
        }
    }
}

#[cfg(feature = "backend_rapier")]
pub type DefaultPhysicsBackend = rapier_backend::RapierPhysicsBackend;

#[cfg(not(feature = "backend_rapier"))]
pub type DefaultPhysicsBackend = LocalPhysicsBackend;
