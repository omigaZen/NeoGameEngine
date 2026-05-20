use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use crate::body::{BodyDesc, BodyKind, ForceMode, MassDesc, Velocity};
use crate::character::{
    CharacterCollision, CharacterControllerDesc, CharacterMoveInput, CharacterMoveOutput,
};
use crate::collider::{ActiveEvents, Axis3, ColliderDesc, ColliderShape, CompoundShapePart};
use crate::command::{CommandApplyReport, PhysicsCommand, PhysicsCommandBuffer};
use crate::config::{PhysicsConfig, PhysicsFrameReport, PhysicsStepReport};
use crate::debug::{
    DebugDrawCategory, DebugLineStyle, DebugShapeStyle, PhysicsDebugDrawOptions,
    PhysicsDebugRenderer,
};
use crate::ecs::{PhysicsSyncComponent, PhysicsSyncMode};
use crate::error::{PhysicsError, PhysicsResult};
use crate::event::{
    ordered_pair, CollisionEvent, ContactForceEvent, ContactManifold, ContactPoint, EventCursor,
    EventDropped, PhysicsEvent, SensorEvent,
};
use crate::filter::QueryFilter;
use crate::hooks::{
    CollisionDecision, CollisionPairInfo, ContactModificationContext, PhysicsHooks,
};
use crate::id::{
    BodyId, CharacterControllerId, ColliderId, JointId, PhysicsMeshId, PhysicsTick,
    PhysicsUserData, Slot,
};
use crate::joint::{JointAxis, JointDesc, JointLimits, JointMotor};
use crate::material::PhysicsMaterial;
use crate::math::{Aabb, Quat, Real, Transform, Vec3};
use crate::mesh::{ConvexMeshDesc, HeightFieldDesc, PhysicsMeshDesc, TriMeshDesc};
use crate::query::{
    OverlapHit, OverlapInput, PhysicsQuery, PhysicsQuerySnapshot, PointProjection, QueryGizmo,
    QueryGizmoKind, Ray, RayHit, ShapeCastHit, ShapeCastInput,
};
use crate::snapshot::{
    BodySnapshot, CharacterControllerSnapshot, ColliderSnapshot, JointSnapshot,
    PhysicsMeshSnapshot, PhysicsSnapshot,
};

#[derive(Clone, Debug)]
struct BodyState {
    desc: BodyDesc,
    transform: Transform,
    previous_transform: Transform,
    velocity: Velocity,
    force_accumulator: Vec3,
    acceleration_accumulator: Vec3,
    torque_accumulator: Vec3,
    angular_acceleration_accumulator: Vec3,
    next_kinematic_transform: Option<Transform>,
    mass: Real,
    sleeping: bool,
    sleep_timer: Real,
}

impl BodyState {
    fn new(desc: BodyDesc) -> Self {
        let mass = mass_value(desc.mass);
        Self {
            previous_transform: desc.transform,
            transform: desc.transform,
            velocity: desc.velocity,
            desc,
            force_accumulator: Vec3::ZERO,
            acceleration_accumulator: Vec3::ZERO,
            torque_accumulator: Vec3::ZERO,
            angular_acceleration_accumulator: Vec3::ZERO,
            next_kinematic_transform: None,
            mass,
            sleeping: false,
            sleep_timer: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
struct ColliderState {
    parent: Option<BodyId>,
    desc: ColliderDesc,
}

#[derive(Clone, Debug)]
struct MeshState {
    desc: PhysicsMeshDesc,
}

#[derive(Clone, Debug)]
struct JointState {
    body_a: BodyId,
    body_b: BodyId,
    desc: JointDesc,
    enabled: bool,
}

#[derive(Clone, Debug)]
struct CharacterControllerState {
    desc: CharacterControllerDesc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PairState {
    a: ColliderId,
    b: ColliderId,
    sensor: bool,
}

pub struct PhysicsWorld {
    config: PhysicsConfig,
    tick: PhysicsTick,
    frame_index: u64,
    accumulator: Real,

    bodies: Vec<Slot<BodyState>>,
    free_bodies: Vec<u32>,
    colliders: Vec<Slot<ColliderState>>,
    free_colliders: Vec<u32>,
    meshes: Vec<Slot<MeshState>>,
    free_meshes: Vec<u32>,
    joints: Vec<Slot<JointState>>,
    free_joints: Vec<u32>,
    character_controllers: Vec<Slot<CharacterControllerState>>,
    free_character_controllers: Vec<u32>,

    events: Vec<PhysicsEvent>,
    current_pairs: BTreeMap<(u64, u64), PairState>,
    contact_manifolds: BTreeMap<(u64, u64), ContactManifold>,
    command_buffer: PhysicsCommandBuffer,
    query_gizmos: RefCell<Vec<QueryGizmo>>,
    hooks: Option<Box<dyn PhysicsHooks>>,
    dropped_events_total: usize,
}

impl PhysicsWorld {
    pub fn new(config: PhysicsConfig) -> Self {
        Self {
            config,
            tick: PhysicsTick(0),
            frame_index: 0,
            accumulator: 0.0,
            bodies: Vec::new(),
            free_bodies: Vec::new(),
            colliders: Vec::new(),
            free_colliders: Vec::new(),
            meshes: Vec::new(),
            free_meshes: Vec::new(),
            joints: Vec::new(),
            free_joints: Vec::new(),
            character_controllers: Vec::new(),
            free_character_controllers: Vec::new(),
            events: Vec::new(),
            current_pairs: BTreeMap::new(),
            contact_manifolds: BTreeMap::new(),
            command_buffer: PhysicsCommandBuffer::new(),
            query_gizmos: RefCell::new(Vec::new()),
            hooks: None,
            dropped_events_total: 0,
        }
    }

    pub fn config(&self) -> &PhysicsConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut PhysicsConfig {
        &mut self.config
    }

    pub fn gravity(&self) -> Vec3 {
        self.config.gravity
    }

    pub fn set_gravity(&mut self, gravity: Vec3) {
        self.config.gravity = gravity;
    }

    pub fn tick(&self) -> PhysicsTick {
        self.tick
    }

    pub fn frame_index(&self) -> u64 {
        self.frame_index
    }

    pub fn step_fixed(&mut self, dt: Real) -> PhysicsStepReport {
        let dt = if dt.is_finite() && dt > 0.0 {
            dt
        } else {
            self.config.timestep.fixed_dt
        };
        let command_report = self.apply_queued_commands();

        self.tick.0 += 1;
        let tick = self.tick;
        self.integrate_bodies(dt);
        self.solve_joints();
        let generated_events = self.detect_and_resolve_collisions(tick);
        self.clear_accumulators();
        let active_bodies = self
            .bodies_iter()
            .filter(|(_, body)| body.desc.enabled && !body.sleeping)
            .count();

        PhysicsStepReport {
            tick,
            dt,
            active_bodies,
            events_generated: generated_events,
            commands_applied: command_report.applied,
        }
    }

    pub fn update_fixed(&mut self, frame_dt: Real) -> PhysicsFrameReport {
        self.frame_index += 1;
        let frame_dt = if frame_dt.is_finite() && frame_dt > 0.0 {
            frame_dt.min(self.config.timestep.max_frame_dt)
        } else {
            0.0
        };
        self.accumulator += frame_dt;

        let fixed_dt = self.config.timestep.fixed_dt.max(Real::EPSILON);
        let mut steps_run = 0;
        while self.accumulator + Real::EPSILON >= fixed_dt
            && steps_run < self.config.timestep.max_substeps
        {
            self.step_fixed(fixed_dt);
            self.accumulator -= fixed_dt;
            steps_run += 1;
        }

        let mut dropped_steps = 0;
        while self.accumulator + Real::EPSILON >= fixed_dt {
            self.accumulator -= fixed_dt;
            dropped_steps += 1;
        }

        PhysicsFrameReport {
            frame_index: self.frame_index,
            frame_dt,
            steps_run,
            dropped_steps,
            accumulator: self.accumulator,
            interpolation_alpha: self.interpolation_alpha(),
        }
    }

    pub fn reset_accumulator(&mut self) {
        self.accumulator = 0.0;
    }

    pub fn interpolation_alpha(&self) -> Real {
        let fixed_dt = self.config.timestep.fixed_dt.max(Real::EPSILON);
        (self.accumulator / fixed_dt).clamp(0.0, 1.0)
    }

    pub fn create_body(&mut self, desc: BodyDesc) -> PhysicsResult<BodyId> {
        validate_transform(desc.transform)?;
        validate_mass(desc.mass)?;
        Ok(self.allocate_body(BodyState::new(desc)))
    }

    pub fn destroy_body(&mut self, body: BodyId) -> PhysicsResult<()> {
        self.destroy_body_recursive(body).map(|_| ())
    }

    pub fn destroy_body_recursive(&mut self, body: BodyId) -> PhysicsResult<DestroyedObjects> {
        self.body_state(body)?;
        let mut destroyed = DestroyedObjects::default();
        let colliders: Vec<_> = self
            .colliders_iter()
            .filter_map(|(id, collider)| (collider.parent == Some(body)).then_some(id))
            .collect();
        for collider in colliders {
            self.destroy_collider(collider)?;
            destroyed.colliders.push(collider);
        }

        let joints: Vec<_> = self
            .joints_iter()
            .filter_map(|(id, joint)| (joint.body_a == body || joint.body_b == body).then_some(id))
            .collect();
        for joint in joints {
            self.destroy_joint(joint)?;
            destroyed.joints.push(joint);
        }

        self.release_body(body)?;
        destroyed.bodies.push(body);
        Ok(destroyed)
    }

    pub fn contains_body(&self, body: BodyId) -> bool {
        self.body_state(body).is_ok()
    }

    pub fn body_kind(&self, body: BodyId) -> PhysicsResult<BodyKind> {
        Ok(self.body_state(body)?.desc.kind)
    }

    pub fn body_transform(&self, body: BodyId) -> PhysicsResult<Transform> {
        Ok(self.body_state(body)?.transform)
    }

    pub fn body_previous_transform(&self, body: BodyId) -> PhysicsResult<Transform> {
        Ok(self.body_state(body)?.previous_transform)
    }

    pub fn body_interpolated_transform(
        &self,
        body: BodyId,
        alpha: Real,
    ) -> PhysicsResult<Transform> {
        let body = self.body_state(body)?;
        Ok(body
            .previous_transform
            .interpolate(body.transform, alpha.clamp(0.0, 1.0)))
    }

    pub fn body_velocity(&self, body: BodyId) -> PhysicsResult<Velocity> {
        Ok(self.body_state(body)?.velocity)
    }

    pub fn body_mass(&self, body: BodyId) -> PhysicsResult<Real> {
        Ok(self.body_state(body)?.mass)
    }

    pub fn body_center_of_mass(&self, body: BodyId) -> PhysicsResult<Vec3> {
        match self.body_state(body)?.desc.mass {
            MassDesc::Explicit { center_of_mass, .. } => Ok(center_of_mass),
            _ => Ok(Vec3::ZERO),
        }
    }

    pub fn body_is_sleeping(&self, body: BodyId) -> PhysicsResult<bool> {
        Ok(self.body_state(body)?.sleeping)
    }

    pub fn body_is_enabled(&self, body: BodyId) -> PhysicsResult<bool> {
        Ok(self.body_state(body)?.desc.enabled)
    }

    pub fn body_user_data(&self, body: BodyId) -> PhysicsResult<PhysicsUserData> {
        Ok(self.body_state(body)?.desc.user_data)
    }

    pub fn set_body_transform(
        &mut self,
        body: BodyId,
        transform: Transform,
        wake_up: bool,
    ) -> PhysicsResult<()> {
        validate_transform(transform)?;
        let body = self.body_state_mut(body)?;
        body.previous_transform = body.transform;
        body.transform = transform;
        body.desc.transform = transform;
        if wake_up {
            body.sleeping = false;
        }
        Ok(())
    }

    pub fn set_next_kinematic_transform(
        &mut self,
        body: BodyId,
        next_transform: Transform,
    ) -> PhysicsResult<()> {
        validate_transform(next_transform)?;
        let state = self.body_state_mut(body)?;
        if state.desc.kind != BodyKind::KinematicPosition {
            return Err(PhysicsError::Unsupported(
                "set_next_kinematic_transform requires a KinematicPosition body",
            ));
        }
        state.next_kinematic_transform = Some(next_transform);
        state.sleeping = false;
        Ok(())
    }

    pub fn set_body_velocity(
        &mut self,
        body: BodyId,
        velocity: Velocity,
        wake_up: bool,
    ) -> PhysicsResult<()> {
        validate_vec3(velocity.linear)?;
        validate_vec3(velocity.angular)?;
        let body = self.body_state_mut(body)?;
        body.velocity = velocity;
        body.desc.velocity = velocity;
        if wake_up {
            body.sleeping = false;
        }
        Ok(())
    }

    pub fn set_body_linear_velocity(
        &mut self,
        body: BodyId,
        linear: Vec3,
        wake_up: bool,
    ) -> PhysicsResult<()> {
        let mut velocity = self.body_velocity(body)?;
        velocity.linear = linear;
        self.set_body_velocity(body, velocity, wake_up)
    }

    pub fn set_body_angular_velocity(
        &mut self,
        body: BodyId,
        angular: Vec3,
        wake_up: bool,
    ) -> PhysicsResult<()> {
        let mut velocity = self.body_velocity(body)?;
        velocity.angular = angular;
        self.set_body_velocity(body, velocity, wake_up)
    }

    pub fn set_body_enabled(&mut self, body: BodyId, enabled: bool) -> PhysicsResult<()> {
        self.body_state_mut(body)?.desc.enabled = enabled;
        Ok(())
    }

    pub fn set_body_kind(&mut self, body: BodyId, kind: BodyKind) -> PhysicsResult<()> {
        let state = self.body_state_mut(body)?;
        state.desc.kind = kind;
        if kind != BodyKind::Dynamic {
            state.mass = Real::INFINITY;
            state.desc.mass = MassDesc::Infinite;
        } else if matches!(state.desc.mass, MassDesc::Infinite) {
            state.desc.mass = MassDesc::Auto;
            state.mass = 1.0;
        }
        Ok(())
    }

    pub fn wake_body(&mut self, body: BodyId) -> PhysicsResult<()> {
        let body = self.body_state_mut(body)?;
        body.sleeping = false;
        body.sleep_timer = 0.0;
        Ok(())
    }

    pub fn sleep_body(&mut self, body: BodyId) -> PhysicsResult<()> {
        self.body_state_mut(body)?.sleeping = true;
        Ok(())
    }

    pub fn add_force(
        &mut self,
        body: BodyId,
        force: Vec3,
        mode: ForceMode,
        wake_up: bool,
    ) -> PhysicsResult<()> {
        validate_vec3(force)?;
        let body = self.body_state_mut(body)?;
        if body.desc.kind != BodyKind::Dynamic {
            return Err(PhysicsError::Unsupported("forces require a Dynamic body"));
        }
        match mode {
            ForceMode::Force => body.force_accumulator += force,
            ForceMode::Acceleration => body.acceleration_accumulator += force,
            ForceMode::Impulse => body.velocity.linear += force / body.mass.max(Real::EPSILON),
            ForceMode::VelocityChange => body.velocity.linear += force,
        }
        if wake_up {
            body.sleeping = false;
        }
        Ok(())
    }

    pub fn add_force_at_point(
        &mut self,
        body: BodyId,
        force: Vec3,
        world_point: Vec3,
        mode: ForceMode,
        wake_up: bool,
    ) -> PhysicsResult<()> {
        let center = self.body_transform(body)?.translation;
        let torque = (world_point - center).cross(force);
        self.add_force(body, force, mode, wake_up)?;
        self.add_torque(body, torque, mode, wake_up)
    }

    pub fn add_torque(
        &mut self,
        body: BodyId,
        torque: Vec3,
        mode: ForceMode,
        wake_up: bool,
    ) -> PhysicsResult<()> {
        validate_vec3(torque)?;
        let body = self.body_state_mut(body)?;
        if body.desc.kind != BodyKind::Dynamic {
            return Err(PhysicsError::Unsupported("torques require a Dynamic body"));
        }
        match mode {
            ForceMode::Force => body.torque_accumulator += torque,
            ForceMode::Acceleration => body.angular_acceleration_accumulator += torque,
            ForceMode::Impulse => body.velocity.angular += torque / body.mass.max(Real::EPSILON),
            ForceMode::VelocityChange => body.velocity.angular += torque,
        }
        if wake_up {
            body.sleeping = false;
        }
        Ok(())
    }

    pub fn clear_forces(&mut self, body: BodyId) -> PhysicsResult<()> {
        let body = self.body_state_mut(body)?;
        body.force_accumulator = Vec3::ZERO;
        body.acceleration_accumulator = Vec3::ZERO;
        body.torque_accumulator = Vec3::ZERO;
        body.angular_acceleration_accumulator = Vec3::ZERO;
        Ok(())
    }

    pub fn create_trimesh(&mut self, desc: TriMeshDesc) -> PhysicsResult<PhysicsMeshId> {
        validate_trimesh(&desc)?;
        Ok(self.allocate_mesh(MeshState {
            desc: PhysicsMeshDesc::TriMesh(desc),
        }))
    }

    pub fn create_convex_mesh(&mut self, desc: ConvexMeshDesc) -> PhysicsResult<PhysicsMeshId> {
        validate_convex_mesh(&desc)?;
        Ok(self.allocate_mesh(MeshState {
            desc: PhysicsMeshDesc::Convex(desc),
        }))
    }

    pub fn create_heightfield(&mut self, desc: HeightFieldDesc) -> PhysicsResult<PhysicsMeshId> {
        validate_heightfield(&desc)?;
        Ok(self.allocate_mesh(MeshState {
            desc: PhysicsMeshDesc::HeightField(desc),
        }))
    }

    pub fn destroy_mesh(&mut self, mesh: PhysicsMeshId) -> PhysicsResult<()> {
        self.mesh_state(mesh)?;
        let in_use = self
            .colliders_iter()
            .any(|(_, collider)| shape_references_mesh(&collider.desc.shape, mesh));
        if in_use {
            return Err(PhysicsError::Backend(format!(
                "mesh {:?} is still referenced by a collider",
                mesh
            )));
        }
        self.release_mesh(mesh)
    }

    pub fn contains_mesh(&self, mesh: PhysicsMeshId) -> bool {
        self.mesh_state(mesh).is_ok()
    }

    pub fn create_collider(&mut self, desc: ColliderDesc) -> PhysicsResult<ColliderId> {
        self.create_collider_internal(None, desc)
    }

    pub fn create_collider_with_parent(
        &mut self,
        parent: BodyId,
        desc: ColliderDesc,
    ) -> PhysicsResult<ColliderId> {
        self.body_state(parent)?;
        self.create_collider_internal(Some(parent), desc)
    }

    pub fn attach_collider(&mut self, collider: ColliderId, parent: BodyId) -> PhysicsResult<()> {
        self.body_state(parent)?;
        self.collider_state_mut(collider)?.parent = Some(parent);
        self.recompute_body_mass(parent);
        Ok(())
    }

    pub fn detach_collider(&mut self, collider: ColliderId) -> PhysicsResult<()> {
        let parent = self.collider_state(collider)?.parent;
        self.collider_state_mut(collider)?.parent = None;
        if let Some(parent) = parent {
            self.recompute_body_mass(parent);
        }
        Ok(())
    }

    pub fn destroy_collider(&mut self, collider: ColliderId) -> PhysicsResult<()> {
        let parent = self.collider_state(collider)?.parent;
        self.release_collider(collider)?;
        self.current_pairs
            .retain(|_, pair| pair.a != collider && pair.b != collider);
        self.contact_manifolds
            .retain(|_, manifold| manifold.a != collider && manifold.b != collider);
        if let Some(parent) = parent {
            self.recompute_body_mass(parent);
        }
        Ok(())
    }

    pub fn contains_collider(&self, collider: ColliderId) -> bool {
        self.collider_state(collider).is_ok()
    }

    pub fn collider_parent(&self, collider: ColliderId) -> PhysicsResult<Option<BodyId>> {
        Ok(self.collider_state(collider)?.parent)
    }

    pub fn collider_shape(&self, collider: ColliderId) -> PhysicsResult<ColliderShape> {
        Ok(self.collider_state(collider)?.desc.shape.clone())
    }

    pub fn collider_world_transform(&self, collider: ColliderId) -> PhysicsResult<Transform> {
        self.collider_world_transform_internal(collider)
    }

    pub fn collider_local_transform(&self, collider: ColliderId) -> PhysicsResult<Transform> {
        Ok(self.collider_state(collider)?.desc.local_transform)
    }

    pub fn collider_material(&self, collider: ColliderId) -> PhysicsResult<PhysicsMaterial> {
        Ok(self.collider_state(collider)?.desc.material)
    }

    pub fn collider_filter(
        &self,
        collider: ColliderId,
    ) -> PhysicsResult<crate::filter::CollisionFilter> {
        Ok(self.collider_state(collider)?.desc.filter)
    }

    pub fn collider_is_sensor(&self, collider: ColliderId) -> PhysicsResult<bool> {
        Ok(self.collider_state(collider)?.desc.sensor)
    }

    pub fn set_collider_shape(
        &mut self,
        collider: ColliderId,
        shape: ColliderShape,
    ) -> PhysicsResult<()> {
        self.validate_shape(&shape)?;
        let parent = self.collider_state(collider)?.parent;
        self.collider_state_mut(collider)?.desc.shape = shape;
        if let Some(parent) = parent {
            self.recompute_body_mass(parent);
        }
        Ok(())
    }

    pub fn set_collider_local_transform(
        &mut self,
        collider: ColliderId,
        transform: Transform,
    ) -> PhysicsResult<()> {
        validate_transform(transform)?;
        self.collider_state_mut(collider)?.desc.local_transform = transform;
        Ok(())
    }

    pub fn set_collider_material(
        &mut self,
        collider: ColliderId,
        material: PhysicsMaterial,
    ) -> PhysicsResult<()> {
        self.collider_state_mut(collider)?.desc.material = material;
        Ok(())
    }

    pub fn set_collider_filter(
        &mut self,
        collider: ColliderId,
        filter: crate::filter::CollisionFilter,
    ) -> PhysicsResult<()> {
        self.collider_state_mut(collider)?.desc.filter = filter;
        Ok(())
    }

    pub fn set_collider_sensor(&mut self, collider: ColliderId, sensor: bool) -> PhysicsResult<()> {
        self.collider_state_mut(collider)?.desc.sensor = sensor;
        Ok(())
    }

    pub fn set_collider_enabled(
        &mut self,
        collider: ColliderId,
        enabled: bool,
    ) -> PhysicsResult<()> {
        self.collider_state_mut(collider)?.desc.enabled = enabled;
        Ok(())
    }

    pub fn queue_command(&mut self, command: PhysicsCommand) {
        self.command_buffer.push(command);
    }

    pub fn apply_commands(&mut self, commands: &mut PhysicsCommandBuffer) -> CommandApplyReport {
        let mut report = CommandApplyReport::default();
        for command in commands.drain() {
            match self.apply_command(command) {
                Ok(()) => report.applied += 1,
                Err(err) => {
                    report.failed += 1;
                    report.errors.push(err);
                }
            }
        }
        report
    }

    pub fn events(&self) -> &[PhysicsEvent] {
        &self.events
    }

    pub fn drain_events(&mut self) -> std::vec::Drain<'_, PhysicsEvent> {
        self.events.drain(..)
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn events_since(&self, cursor: &mut EventCursor) -> &[PhysicsEvent] {
        let start = cursor.index.min(self.events.len());
        cursor.index = self.events.len();
        &self.events[start..]
    }

    pub fn dropped_event_count(&self) -> usize {
        self.dropped_events_total
    }

    pub fn query(&self) -> PhysicsQuery<'_> {
        PhysicsQuery::new(self)
    }

    pub fn query_snapshot(&self) -> PhysicsQuerySnapshot {
        let mut hits = Vec::new();
        for (collider, state) in self.colliders_iter() {
            hits.push(OverlapHit {
                collider,
                body: state.parent,
                user_data: state.desc.user_data,
            });
        }
        PhysicsQuerySnapshot { hits }
    }

    pub fn contact_pair(
        &self,
        a: ColliderId,
        b: ColliderId,
    ) -> PhysicsResult<Option<ContactManifold>> {
        self.collider_state(a)?;
        self.collider_state(b)?;
        Ok(self.contact_manifolds.get(&ordered_pair(a, b)).cloned())
    }

    pub fn contacts_with_body(
        &self,
        body: BodyId,
        out: &mut Vec<ContactManifold>,
    ) -> PhysicsResult<usize> {
        self.body_state(body)?;
        let start_len = out.len();
        out.extend(
            self.contact_manifolds
                .values()
                .filter(|manifold| manifold.body_a == Some(body) || manifold.body_b == Some(body))
                .cloned(),
        );
        Ok(out.len() - start_len)
    }

    pub fn contacts_with_collider(
        &self,
        collider: ColliderId,
        out: &mut Vec<ContactManifold>,
    ) -> PhysicsResult<usize> {
        self.collider_state(collider)?;
        let start_len = out.len();
        out.extend(
            self.contact_manifolds
                .values()
                .filter(|manifold| manifold.a == collider || manifold.b == collider)
                .cloned(),
        );
        Ok(out.len() - start_len)
    }

    pub fn create_character_controller(
        &mut self,
        desc: CharacterControllerDesc,
    ) -> CharacterControllerId {
        self.allocate_character_controller(CharacterControllerState { desc })
    }

    pub fn destroy_character_controller(&mut self, id: CharacterControllerId) -> PhysicsResult<()> {
        self.release_character_controller(id)
    }

    pub fn character_controller(
        &self,
        id: CharacterControllerId,
    ) -> PhysicsResult<&CharacterControllerDesc> {
        Ok(&self.character_controller_state(id)?.desc)
    }

    pub fn character_controller_mut(
        &mut self,
        id: CharacterControllerId,
    ) -> PhysicsResult<&mut CharacterControllerDesc> {
        Ok(&mut self.character_controller_state_mut(id)?.desc)
    }

    pub fn compute_character_movement(
        &self,
        input: CharacterMoveInput,
    ) -> PhysicsResult<CharacterMoveOutput> {
        let controller = self.character_controller(input.controller)?.clone();
        let body_transform = self.body_transform(input.body)?;
        self.collider_state(input.collider)?;
        let mut filter = input.filter;
        filter.exclude_body = Some(input.body);
        filter.exclude_collider = Some(input.collider);

        let shape = self.collider_shape(input.collider)?;
        let cast = ShapeCastInput {
            shape,
            transform: body_transform,
            translation: input.desired_translation,
            max_toi: 1.0,
            stop_at_penetration: true,
            target_distance: controller.offset,
        };
        let mut corrected = input.desired_translation;
        let mut hit_wall = false;
        let mut hit_ceiling = false;
        let mut collisions = Vec::new();
        if let Some(hit) = self.cast_shape_internal(cast, filter) {
            let allowed = (hit.toi - controller.offset).max(0.0).min(1.0);
            corrected = input.desired_translation * allowed;
            let normal = hit.normal2;
            if controller.enable_slide {
                let remaining = input.desired_translation - corrected;
                let slide = remaining - normal * remaining.dot(normal);
                corrected += slide;
            }
            hit_wall = normal.y.abs() < 0.5;
            hit_ceiling = normal.y < -0.5;
            collisions.push(CharacterCollision {
                collider: hit.collider,
                body: hit.body,
                point: hit.point2,
                normal,
                translation_remaining: input.desired_translation - corrected,
            });
        }

        let mut final_transform = body_transform;
        final_transform.translation += corrected;

        let mut grounded = false;
        let mut ground_collider = None;
        let mut ground_body = None;
        let mut ground_normal = controller.up;
        if controller.enable_snap_to_ground {
            let down = -controller.up.normalize_or_zero()
                * controller.snap_to_ground_distance.max(controller.offset);
            let ground_cast = ShapeCastInput {
                shape: self.collider_shape(input.collider)?,
                transform: final_transform,
                translation: down,
                max_toi: 1.0,
                stop_at_penetration: true,
                target_distance: controller.offset,
            };
            if let Some(hit) = self.cast_shape_internal(ground_cast, filter) {
                grounded = hit.normal2.dot(controller.up.normalize_or_zero())
                    >= controller.max_slope_angle.cos();
                ground_collider = Some(hit.collider);
                ground_body = hit.body;
                ground_normal = hit.normal2;
                if grounded {
                    final_transform.translation += down * hit.toi;
                }
            }
        }

        Ok(CharacterMoveOutput {
            requested_translation: input.desired_translation,
            corrected_translation: final_transform.translation - body_transform.translation,
            final_transform,
            grounded,
            ground_collider,
            ground_body,
            ground_normal,
            hit_wall,
            hit_ceiling,
            collisions,
        })
    }

    pub fn move_character(
        &mut self,
        input: CharacterMoveInput,
    ) -> PhysicsResult<CharacterMoveOutput> {
        let output = self.compute_character_movement(input.clone())?;
        let controller = self.character_controller(input.controller)?.clone();
        self.set_body_transform(input.body, output.final_transform, true)?;
        if controller.apply_impulses_to_dynamic_bodies {
            for collision in &output.collisions {
                if let Some(body) = collision.body {
                    if self.body_kind(body).ok() == Some(BodyKind::Dynamic) {
                        let impulse = input.desired_translation.normalize_or_zero()
                            * input.desired_translation.length()
                            / input.dt.max(Real::EPSILON);
                        let _ = self.add_force(body, impulse, ForceMode::VelocityChange, true);
                    }
                }
            }
        }
        Ok(output)
    }

    pub fn create_joint(
        &mut self,
        body_a: BodyId,
        body_b: BodyId,
        desc: JointDesc,
    ) -> PhysicsResult<JointId> {
        self.body_state(body_a)?;
        self.body_state(body_b)?;
        validate_joint_desc(&desc)?;
        Ok(self.allocate_joint(JointState {
            body_a,
            body_b,
            desc,
            enabled: true,
        }))
    }

    pub fn destroy_joint(&mut self, joint: JointId) -> PhysicsResult<()> {
        self.release_joint(joint)
    }

    pub fn contains_joint(&self, joint: JointId) -> bool {
        self.joint_state(joint).is_ok()
    }

    pub fn joint_bodies(&self, joint: JointId) -> PhysicsResult<(BodyId, BodyId)> {
        let joint = self.joint_state(joint)?;
        Ok((joint.body_a, joint.body_b))
    }

    pub fn set_joint_enabled(&mut self, joint: JointId, enabled: bool) -> PhysicsResult<()> {
        self.joint_state_mut(joint)?.enabled = enabled;
        Ok(())
    }

    pub fn set_joint_motor(
        &mut self,
        joint: JointId,
        axis: JointAxis,
        motor: JointMotor,
    ) -> PhysicsResult<()> {
        validate_motor(motor)?;
        let joint = self.joint_state_mut(joint)?;
        set_joint_motor_on_desc(&mut joint.desc, axis, motor);
        Ok(())
    }

    pub fn set_joint_limits(
        &mut self,
        joint: JointId,
        axis: JointAxis,
        limits: JointLimits,
    ) -> PhysicsResult<()> {
        validate_limits(limits)?;
        let joint = self.joint_state_mut(joint)?;
        set_joint_limits_on_desc(&mut joint.desc, axis, limits);
        Ok(())
    }

    pub fn debug_draw(
        &self,
        renderer: &mut dyn PhysicsDebugRenderer,
        options: PhysicsDebugDrawOptions,
    ) {
        if !self.config.debug.enabled {
            return;
        }
        if options.draw_bodies {
            for (_, body) in self.bodies_iter() {
                let category = match body.desc.kind {
                    BodyKind::Dynamic => DebugDrawCategory::DynamicBody,
                    BodyKind::Fixed => DebugDrawCategory::FixedBody,
                    BodyKind::KinematicPosition | BodyKind::KinematicVelocity => {
                        DebugDrawCategory::KinematicBody
                    }
                };
                let category = if options.draw_sleeping && body.sleeping {
                    DebugDrawCategory::Sleeping
                } else {
                    category
                };
                renderer.sphere(
                    body.transform.translation,
                    0.08,
                    DebugShapeStyle::new(category),
                );
                if options.draw_names {
                    if let Some(name) = &body.desc.debug_name {
                        renderer.text(body.transform.translation, name);
                    }
                }
            }
        }
        if options.draw_colliders {
            for (id, collider) in self.colliders_iter() {
                if !collider.desc.enabled {
                    continue;
                }
                if let Ok(transform) = self.collider_world_transform_internal(id) {
                    let category = if collider.desc.sensor {
                        DebugDrawCategory::Sensor
                    } else {
                        collider
                            .parent
                            .and_then(|body| self.body_state(body).ok())
                            .map(|body| match body.desc.kind {
                                BodyKind::Dynamic => DebugDrawCategory::DynamicBody,
                                BodyKind::Fixed => DebugDrawCategory::FixedBody,
                                BodyKind::KinematicPosition | BodyKind::KinematicVelocity => {
                                    DebugDrawCategory::KinematicBody
                                }
                            })
                            .unwrap_or(DebugDrawCategory::FixedBody)
                    };
                    self.debug_draw_shape(renderer, &collider.desc.shape, transform, category);
                    if options.draw_aabbs {
                        if let Some(aabb) = self.shape_aabb(&collider.desc.shape, transform) {
                            draw_aabb(renderer, aabb, DebugDrawCategory::Query);
                        }
                    }
                    if options.draw_names {
                        if let Some(name) = &collider.desc.debug_name {
                            renderer.text(transform.translation, name);
                        }
                    }
                }
            }
        }
        if options.draw_contacts {
            for manifold in self.contact_manifolds.values() {
                for contact in &manifold.contacts {
                    renderer.sphere(
                        contact.position,
                        0.04,
                        DebugShapeStyle::new(DebugDrawCategory::Contact),
                    );
                    if options.draw_contact_normals {
                        renderer.line(
                            contact.position,
                            contact.position + contact.normal * 0.5,
                            DebugLineStyle::new(DebugDrawCategory::Contact),
                        );
                    }
                }
            }
        }
        if options.draw_joints {
            for (_, joint) in self.joints_iter() {
                if !joint.enabled {
                    continue;
                }
                if let (Ok(a), Ok(b)) = (
                    self.body_transform(joint.body_a),
                    self.body_transform(joint.body_b),
                ) {
                    renderer.line(
                        a.translation,
                        b.translation,
                        DebugLineStyle::new(DebugDrawCategory::Joint),
                    );
                }
            }
        }
        if options.draw_query_gizmos {
            for gizmo in self.query_gizmos.borrow().iter() {
                renderer.line(
                    gizmo.from,
                    gizmo.to,
                    DebugLineStyle::new(DebugDrawCategory::Query),
                );
                if let Some(hit) = gizmo.hit {
                    renderer.sphere(hit, 0.05, DebugShapeStyle::new(DebugDrawCategory::Query));
                }
            }
        }
    }

    pub fn snapshot(&self) -> PhysicsSnapshot {
        PhysicsSnapshot {
            tick: self.tick,
            frame_index: self.frame_index,
            accumulator: self.accumulator,
            config: self.config.clone(),
            bodies: self
                .bodies_iter()
                .map(|(id, body)| BodySnapshot {
                    id,
                    desc: body.desc.clone(),
                    transform: body.transform,
                    previous_transform: body.previous_transform,
                    velocity: body.velocity,
                    sleeping: body.sleeping,
                })
                .collect(),
            colliders: self
                .colliders_iter()
                .map(|(id, collider)| ColliderSnapshot {
                    id,
                    parent: collider.parent,
                    desc: collider.desc.clone(),
                })
                .collect(),
            joints: self
                .joints_iter()
                .map(|(id, joint)| JointSnapshot {
                    id,
                    body_a: joint.body_a,
                    body_b: joint.body_b,
                    desc: joint.desc.clone(),
                    enabled: joint.enabled,
                })
                .collect(),
            meshes: self
                .meshes_iter()
                .map(|(id, mesh)| PhysicsMeshSnapshot {
                    id,
                    desc: mesh.desc.clone(),
                })
                .collect(),
            character_controllers: self
                .character_controllers_iter()
                .map(|(id, controller)| CharacterControllerSnapshot {
                    id,
                    desc: controller.desc.clone(),
                })
                .collect(),
        }
    }

    pub fn restore(&mut self, snapshot: PhysicsSnapshot) -> PhysicsResult<()> {
        self.validate_snapshot(&snapshot)?;
        let mut restored = Self::new(snapshot.config.clone());
        restored.tick = snapshot.tick;
        restored.frame_index = snapshot.frame_index;
        restored.accumulator = snapshot.accumulator;

        for mesh in snapshot.meshes {
            restored.insert_mesh_with_id(mesh.id, MeshState { desc: mesh.desc })?;
        }
        for body in snapshot.bodies {
            let mut state = BodyState::new(body.desc);
            state.transform = body.transform;
            state.previous_transform = body.previous_transform;
            state.velocity = body.velocity;
            state.sleeping = body.sleeping;
            state.mass = mass_value(state.desc.mass);
            restored.insert_body_with_id(body.id, state)?;
        }
        for collider in snapshot.colliders {
            restored.insert_collider_with_id(
                collider.id,
                ColliderState {
                    parent: collider.parent,
                    desc: collider.desc,
                },
            )?;
        }
        for joint in snapshot.joints {
            restored.insert_joint_with_id(
                joint.id,
                JointState {
                    body_a: joint.body_a,
                    body_b: joint.body_b,
                    desc: joint.desc,
                    enabled: joint.enabled,
                },
            )?;
        }
        for controller in snapshot.character_controllers {
            restored.insert_character_controller_with_id(
                controller.id,
                CharacterControllerState {
                    desc: controller.desc,
                },
            )?;
        }
        *self = restored;
        Ok(())
    }

    pub fn sync_transforms_to_physics<I>(&mut self, entries: I) -> CommandApplyReport
    where
        I: IntoIterator<Item = (BodyId, Transform, PhysicsSyncComponent)>,
    {
        let mut report = CommandApplyReport::default();
        for (body, transform, sync) in entries {
            if sync.mode != PhysicsSyncMode::TransformToPhysics {
                continue;
            }
            match self.body_kind(body).and_then(|kind| {
                if kind == BodyKind::KinematicPosition {
                    self.set_next_kinematic_transform(body, transform)
                } else {
                    self.set_body_transform(body, transform, true)
                }
            }) {
                Ok(()) => report.applied += 1,
                Err(err) => {
                    report.failed += 1;
                    report.errors.push(err);
                }
            }
        }
        report
    }

    pub fn sync_transforms_from_physics(
        &self,
        entries: &mut [(BodyId, Transform, PhysicsSyncComponent)],
    ) -> CommandApplyReport {
        let mut report = CommandApplyReport::default();
        for (body, transform, sync) in entries {
            if sync.mode != PhysicsSyncMode::PhysicsToTransform {
                continue;
            }
            let result = if sync.interpolate {
                self.body_interpolated_transform(*body, self.interpolation_alpha())
            } else {
                self.body_transform(*body)
            };
            match result {
                Ok(next) => {
                    *transform = next;
                    report.applied += 1;
                }
                Err(err) => {
                    report.failed += 1;
                    report.errors.push(err);
                }
            }
        }
        report
    }

    pub fn set_hooks<H>(&mut self, hooks: H)
    where
        H: PhysicsHooks,
    {
        self.hooks = Some(Box::new(hooks));
    }

    pub fn clear_hooks(&mut self) {
        self.hooks = None;
    }

    pub(crate) fn cast_ray_internal(&self, ray: Ray, filter: QueryFilter) -> Option<RayHit> {
        let mut hits = Vec::new();
        self.cast_ray_all_internal(ray, filter, &mut hits);
        hits.into_iter().next()
    }

    pub(crate) fn cast_ray_all_internal(
        &self,
        ray: Ray,
        filter: QueryFilter,
        out: &mut Vec<RayHit>,
    ) -> usize {
        let start_len = out.len();
        let direction = ray.direction.normalize_or_zero();
        if direction == Vec3::ZERO || ray.max_toi < 0.0 || !ray.origin.is_finite() {
            return 0;
        }
        for (id, collider) in self.colliders_iter() {
            if !self.query_filter_matches(id, collider, filter) {
                continue;
            }
            let Ok(transform) = self.collider_world_transform_internal(id) else {
                continue;
            };
            let Some(aabb) = self.shape_aabb(&collider.desc.shape, transform) else {
                continue;
            };
            if let Some(toi) = ray_aabb(ray.origin, direction, ray.max_toi, aabb) {
                let point = ray.origin + direction * toi;
                out.push(RayHit {
                    collider: id,
                    body: collider.parent,
                    point,
                    normal: aabb_normal_at_point(aabb, point),
                    toi,
                    user_data: collider.desc.user_data,
                });
            }
            if filter
                .max_results
                .is_some_and(|max| out.len() - start_len >= max)
            {
                break;
            }
        }
        out[start_len..].sort_by(|a, b| {
            a.toi
                .partial_cmp(&b.toi)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.collider.raw().cmp(&b.collider.raw()))
        });
        let hit = out[start_len..].first().map(|hit| hit.point);
        self.record_query_gizmo(QueryGizmo {
            kind: QueryGizmoKind::Ray,
            from: ray.origin,
            to: ray.origin + direction * ray.max_toi,
            hit,
        });
        out.len() - start_len
    }

    pub(crate) fn cast_shape_internal(
        &self,
        input: ShapeCastInput,
        filter: QueryFilter,
    ) -> Option<ShapeCastHit> {
        let mut hits = Vec::new();
        self.cast_shape_all_internal(input, filter, &mut hits);
        hits.into_iter().next()
    }

    pub(crate) fn cast_shape_all_internal(
        &self,
        input: ShapeCastInput,
        filter: QueryFilter,
        out: &mut Vec<ShapeCastHit>,
    ) -> usize {
        let start_len = out.len();
        let Some(start_aabb) = self.shape_aabb(&input.shape, input.transform) else {
            return 0;
        };
        let max_toi = input.max_toi.max(0.0);
        let steps = 16;
        for step in 0..=steps {
            let toi = max_toi * (step as Real / steps as Real);
            let transform = Transform {
                translation: input.transform.translation + input.translation * toi,
                ..input.transform
            };
            let Some(shape_aabb) = self.shape_aabb(&input.shape, transform) else {
                continue;
            };
            let mut overlaps = Vec::new();
            self.overlap_aabb_internal(shape_aabb, filter, &mut overlaps);
            for overlap in overlaps {
                let Some((normal, point)) =
                    self.collider_state(overlap.collider)
                        .ok()
                        .and_then(|collider| {
                            self.collider_world_transform_internal(overlap.collider)
                                .ok()
                                .and_then(|t| self.shape_aabb(&collider.desc.shape, t))
                                .map(|other_aabb| {
                                    let normal = (shape_aabb.center() - other_aabb.center())
                                        .normalize_or_zero();
                                    let normal = if normal == Vec3::ZERO {
                                        Vec3::Y
                                    } else {
                                        normal
                                    };
                                    (normal, other_aabb.center())
                                })
                        })
                else {
                    continue;
                };
                out.push(ShapeCastHit {
                    collider: overlap.collider,
                    body: overlap.body,
                    toi,
                    point1: start_aabb.center() + input.translation * toi,
                    point2: point,
                    normal1: -normal,
                    normal2: normal,
                    user_data: overlap.user_data,
                });
                if input.stop_at_penetration
                    || filter
                        .max_results
                        .is_some_and(|max| out.len() - start_len >= max)
                {
                    break;
                }
            }
            if out.len() > start_len {
                break;
            }
        }
        out[start_len..].sort_by(|a, b| {
            a.toi
                .partial_cmp(&b.toi)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.collider.raw().cmp(&b.collider.raw()))
        });
        self.record_query_gizmo(QueryGizmo {
            kind: QueryGizmoKind::ShapeCast,
            from: input.transform.translation,
            to: input.transform.translation + input.translation * max_toi,
            hit: out[start_len..].first().map(|hit| hit.point2),
        });
        out.len() - start_len
    }

    pub(crate) fn overlap_shape_internal(
        &self,
        input: OverlapInput,
        filter: QueryFilter,
        out: &mut Vec<OverlapHit>,
    ) -> usize {
        let Some(aabb) = self.shape_aabb(&input.shape, input.transform) else {
            return 0;
        };
        self.overlap_aabb_internal(aabb, filter, out)
    }

    pub(crate) fn overlap_aabb_internal(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
        out: &mut Vec<OverlapHit>,
    ) -> usize {
        let start_len = out.len();
        for (id, collider) in self.colliders_iter() {
            if !self.query_filter_matches(id, collider, filter) {
                continue;
            }
            let Ok(transform) = self.collider_world_transform_internal(id) else {
                continue;
            };
            let Some(other_aabb) = self.shape_aabb(&collider.desc.shape, transform) else {
                continue;
            };
            if aabb.intersects(other_aabb) {
                out.push(OverlapHit {
                    collider: id,
                    body: collider.parent,
                    user_data: collider.desc.user_data,
                });
            }
            if filter
                .max_results
                .is_some_and(|max| out.len() - start_len >= max)
            {
                break;
            }
        }
        out[start_len..].sort_by_key(|hit| hit.collider.raw());
        self.record_query_gizmo(QueryGizmo {
            kind: QueryGizmoKind::Overlap,
            from: aabb.min,
            to: aabb.max,
            hit: out[start_len..].first().map(|hit| {
                self.collider_world_transform_internal(hit.collider)
                    .map(|t| t.translation)
                    .unwrap_or(Vec3::ZERO)
            }),
        });
        out.len() - start_len
    }

    pub(crate) fn contains_point_internal(
        &self,
        point: Vec3,
        filter: QueryFilter,
        out: &mut Vec<OverlapHit>,
    ) -> usize {
        let start_len = out.len();
        for (id, collider) in self.colliders_iter() {
            if !self.query_filter_matches(id, collider, filter) {
                continue;
            }
            let Ok(transform) = self.collider_world_transform_internal(id) else {
                continue;
            };
            if self
                .shape_aabb(&collider.desc.shape, transform)
                .is_some_and(|aabb| aabb.contains_point(point))
            {
                out.push(OverlapHit {
                    collider: id,
                    body: collider.parent,
                    user_data: collider.desc.user_data,
                });
            }
        }
        out[start_len..].sort_by_key(|hit| hit.collider.raw());
        out.len() - start_len
    }

    pub(crate) fn project_point_internal(
        &self,
        point: Vec3,
        max_distance: Real,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<PointProjection> {
        let mut best: Option<PointProjection> = None;
        for (id, collider) in self.colliders_iter() {
            if !self.query_filter_matches(id, collider, filter) {
                continue;
            }
            let Ok(transform) = self.collider_world_transform_internal(id) else {
                continue;
            };
            let Some(aabb) = self.shape_aabb(&collider.desc.shape, transform) else {
                continue;
            };
            let clamped = point.clamp(aabb.min, aabb.max);
            let is_inside = aabb.contains_point(point);
            if is_inside && !solid {
                continue;
            }
            let distance = point.distance(clamped);
            if distance <= max_distance
                && best
                    .as_ref()
                    .is_none_or(|current| distance < current.distance)
            {
                best = Some(PointProjection {
                    collider: id,
                    body: collider.parent,
                    point: clamped,
                    is_inside,
                    distance,
                    user_data: collider.desc.user_data,
                });
            }
        }
        best
    }

    fn create_collider_internal(
        &mut self,
        parent: Option<BodyId>,
        desc: ColliderDesc,
    ) -> PhysicsResult<ColliderId> {
        validate_transform(desc.local_transform)?;
        self.validate_shape(&desc.shape)?;
        if !desc.density.is_finite() || desc.density < 0.0 {
            return Err(PhysicsError::InvalidShape {
                reason: "collider density must be finite and non-negative".to_owned(),
            });
        }
        let id = self.allocate_collider(ColliderState { parent, desc });
        if let Some(parent) = parent {
            self.recompute_body_mass(parent);
        }
        Ok(id)
    }

    fn integrate_bodies(&mut self, dt: Real) {
        let gravity = self.config.gravity;
        let config = self.config.clone();
        for (_, body) in self.bodies_iter_mut() {
            if !body.desc.enabled {
                continue;
            }
            body.previous_transform = body.transform;
            match body.desc.kind {
                BodyKind::Dynamic => {
                    if body.sleeping {
                        continue;
                    }
                    let inv_mass = if body.mass.is_finite() && body.mass > Real::EPSILON {
                        1.0 / body.mass
                    } else {
                        0.0
                    };
                    let acceleration = gravity * body.desc.gravity_scale
                        + body.force_accumulator * inv_mass
                        + body.acceleration_accumulator;
                    body.velocity.linear += acceleration * dt;
                    body.velocity.angular += (body.torque_accumulator * inv_mass
                        + body.angular_acceleration_accumulator)
                        * dt;
                    apply_locked_angular_velocity(&mut body.velocity.angular, body.desc.lock_axes);
                    let linear_damping = (1.0 - body.desc.damping.linear * dt).clamp(0.0, 1.0);
                    let angular_damping = (1.0 - body.desc.damping.angular * dt).clamp(0.0, 1.0);
                    body.velocity.linear *= linear_damping;
                    body.velocity.angular *= angular_damping;
                    let mut translation_delta = body.velocity.linear * dt;
                    apply_locked_translation(&mut translation_delta, body.desc.lock_axes);
                    body.transform.translation += translation_delta;
                    integrate_rotation(&mut body.transform, body.velocity.angular, dt);
                    body.desc.transform = body.transform;
                    update_sleeping(body, &config, dt);
                }
                BodyKind::KinematicPosition => {
                    if let Some(next) = body.next_kinematic_transform.take() {
                        body.velocity.linear = (next.translation - body.transform.translation) / dt;
                        body.velocity.angular = Vec3::ZERO;
                        body.transform = next;
                        body.desc.transform = next;
                    }
                }
                BodyKind::KinematicVelocity => {
                    let mut translation_delta = body.velocity.linear * dt;
                    apply_locked_translation(&mut translation_delta, body.desc.lock_axes);
                    body.transform.translation += translation_delta;
                    apply_locked_angular_velocity(&mut body.velocity.angular, body.desc.lock_axes);
                    integrate_rotation(&mut body.transform, body.velocity.angular, dt);
                    body.desc.transform = body.transform;
                }
                BodyKind::Fixed => {}
            }
        }
    }

    fn solve_joints(&mut self) {
        let joints: Vec<_> = self
            .joints_iter()
            .filter(|(_, joint)| joint.enabled)
            .map(|(_, joint)| joint.clone())
            .collect();
        for joint in joints {
            self.apply_joint_motor(&joint);
            let Ok(a_transform) = self.body_transform(joint.body_a) else {
                continue;
            };
            let Ok(b_transform) = self.body_transform(joint.body_b) else {
                continue;
            };
            let (min_distance, max_distance) =
                joint_distance_limits(&joint.desc, a_transform, b_transform);
            let delta = b_transform.translation - a_transform.translation;
            let distance = delta.length();
            if distance <= Real::EPSILON {
                continue;
            }
            if distance >= min_distance && distance <= max_distance {
                continue;
            }
            let target = distance.clamp(min_distance, max_distance);
            let correction = delta.normalize_or_zero() * (distance - target);
            self.apply_joint_correction(joint.body_a, joint.body_b, correction);
        }
    }

    fn detect_and_resolve_collisions(&mut self, tick: PhysicsTick) -> usize {
        let colliders: Vec<_> = self
            .colliders_iter()
            .filter_map(|(id, collider)| {
                if collider.desc.enabled {
                    let transform = self.collider_world_transform_internal(id).ok()?;
                    let aabb = self.shape_aabb(&collider.desc.shape, transform)?;
                    Some((id, collider.clone(), aabb))
                } else {
                    None
                }
            })
            .collect();
        let mut new_pairs = BTreeMap::new();
        let mut new_manifolds = BTreeMap::new();
        let mut events = Vec::new();

        for i in 0..colliders.len() {
            for j in (i + 1)..colliders.len() {
                let (a_id, a, aabb) = &colliders[i];
                let (b_id, b, babb) = &colliders[j];
                if a.parent.is_some() && a.parent == b.parent {
                    continue;
                }
                if !a.desc.filter.collides_with(b.desc.filter) || !aabb.intersects(*babb) {
                    continue;
                }
                if !self.parent_body_enabled(a.parent) || !self.parent_body_enabled(b.parent) {
                    continue;
                }
                let pair_info = CollisionPairInfo {
                    collider_a: *a_id,
                    collider_b: *b_id,
                    body_a: a.parent,
                    body_b: b.parent,
                    user_data_a: a.desc.user_data,
                    user_data_b: b.desc.user_data,
                };
                let decision = self
                    .hooks
                    .as_ref()
                    .map(|hooks| hooks.filter_collision_pair(pair_info))
                    .unwrap_or(CollisionDecision::UseDefault);
                if decision == CollisionDecision::DisableCollision {
                    continue;
                }
                let sensor = a.desc.sensor || b.desc.sensor;
                let key = ordered_pair(*a_id, *b_id);
                let pair = PairState {
                    a: *a_id,
                    b: *b_id,
                    sensor,
                };
                new_pairs.insert(key, pair);
                let mut manifold = contact_manifold(*a_id, *b_id, a.parent, b.parent, *aabb, *babb);
                let mut material = a.desc.material;
                if let Some(hooks) = &self.hooks {
                    hooks.modify_contacts(&mut ContactModificationContext {
                        pair: pair_info,
                        contacts: &mut manifold.contacts,
                        material: &mut material,
                    });
                }
                new_manifolds.insert(key, manifold.clone());

                if !self.current_pairs.contains_key(&key) {
                    if sensor && self.config.events.collect_sensor_events {
                        if let Some(event) = sensor_event(tick, *a_id, *b_id, a, b) {
                            events.push(PhysicsEvent::SensorEntered(event));
                        }
                    } else if self.config.events.collect_collision_events
                        && active_collision_events(&a.desc, &b.desc)
                    {
                        events.push(PhysicsEvent::CollisionStarted(CollisionEvent {
                            tick,
                            a: *a_id,
                            b: *b_id,
                            body_a: a.parent,
                            body_b: b.parent,
                        }));
                    }
                }
                if !sensor
                    && self.config.events.collect_contact_force_events
                    && active_force_events(&a.desc, &b.desc)
                {
                    let normal = manifold
                        .contacts
                        .first()
                        .map(|contact| contact.normal)
                        .unwrap_or(Vec3::Y);
                    let rel_vel = relative_velocity(self, a.parent, b.parent);
                    let force_mag = rel_vel.dot(normal).abs()
                        * mass_for_body(self, a.parent)
                            .min(mass_for_body(self, b.parent))
                            .max(1.0);
                    events.push(PhysicsEvent::ContactForce(ContactForceEvent {
                        tick,
                        a: *a_id,
                        b: *b_id,
                        body_a: a.parent,
                        body_b: b.parent,
                        total_force: normal * force_mag,
                        total_force_magnitude: force_mag,
                    }));
                }
                if !sensor && decision != CollisionDecision::DisableSolver {
                    self.resolve_pair(a.parent, b.parent, &manifold, material, b.desc.material);
                }
            }
        }

        for (key, old_pair) in &self.current_pairs {
            if new_pairs.contains_key(key) {
                continue;
            }
            if old_pair.sensor && self.config.events.collect_sensor_events {
                if let (Ok(a), Ok(b)) = (
                    self.collider_state(old_pair.a),
                    self.collider_state(old_pair.b),
                ) {
                    if let Some(event) = sensor_event(tick, old_pair.a, old_pair.b, a, b) {
                        events.push(PhysicsEvent::SensorExited(event));
                    }
                }
            } else if self.config.events.collect_collision_events {
                if let (Ok(a), Ok(b)) = (
                    self.collider_state(old_pair.a),
                    self.collider_state(old_pair.b),
                ) {
                    events.push(PhysicsEvent::CollisionStopped(CollisionEvent {
                        tick,
                        a: old_pair.a,
                        b: old_pair.b,
                        body_a: a.parent,
                        body_b: b.parent,
                    }));
                }
            }
        }

        self.current_pairs = new_pairs;
        self.contact_manifolds = new_manifolds;
        self.emit_events(events)
    }

    fn emit_events(&mut self, mut events: Vec<PhysicsEvent>) -> usize {
        if self.config.determinism.stable_event_sorting {
            events.sort_by(|a, b| {
                a.tick()
                    .cmp(&b.tick())
                    .then_with(|| a.collider_key().cmp(&b.collider_key()))
                    .then_with(|| event_rank(a).cmp(&event_rank(b)))
            });
        }
        let max = self.config.events.max_events_per_tick;
        let generated = events.len();
        if generated > max {
            let dropped = generated - max;
            self.events.extend(events.into_iter().take(max));
            self.events.push(PhysicsEvent::EventDropped(EventDropped {
                tick: self.tick,
                dropped,
                max_events_per_tick: max,
            }));
            self.dropped_events_total += dropped;
        } else {
            self.events.extend(events);
        }
        generated
    }

    fn clear_accumulators(&mut self) {
        for (_, body) in self.bodies_iter_mut() {
            body.force_accumulator = Vec3::ZERO;
            body.acceleration_accumulator = Vec3::ZERO;
            body.torque_accumulator = Vec3::ZERO;
            body.angular_acceleration_accumulator = Vec3::ZERO;
            body.desc.velocity = body.velocity;
        }
    }

    fn apply_queued_commands(&mut self) -> CommandApplyReport {
        let mut commands = std::mem::take(&mut self.command_buffer);
        self.apply_commands(&mut commands)
    }

    fn apply_command(&mut self, command: PhysicsCommand) -> PhysicsResult<()> {
        match command {
            PhysicsCommand::SetBodyTransform {
                body,
                transform,
                wake_up,
            } => self.set_body_transform(body, transform, wake_up),
            PhysicsCommand::SetBodyVelocity {
                body,
                velocity,
                wake_up,
            } => self.set_body_velocity(body, velocity, wake_up),
            PhysicsCommand::AddForce {
                body,
                force,
                mode,
                wake_up,
            } => self.add_force(body, force, mode, wake_up),
            PhysicsCommand::DestroyBody { body, recursive } => {
                if recursive {
                    self.destroy_body_recursive(body).map(|_| ())
                } else {
                    self.destroy_body(body)
                }
            }
            PhysicsCommand::DestroyCollider(collider) => self.destroy_collider(collider),
            PhysicsCommand::DestroyJoint(joint) => self.destroy_joint(joint),
        }
    }

    fn resolve_pair(
        &mut self,
        body_a: Option<BodyId>,
        body_b: Option<BodyId>,
        manifold: &ContactManifold,
        material_a: PhysicsMaterial,
        material_b: PhysicsMaterial,
    ) {
        let Some(contact) = manifold.contacts.first() else {
            return;
        };
        let normal = contact.normal;
        let penetration = contact.penetration.max(0.0);
        let restitution = material_a.combine_restitution(material_b).clamp(0.0, 1.0);

        let a_dynamic = body_a
            .and_then(|id| self.body_state(id).ok())
            .is_some_and(|body| body.desc.kind == BodyKind::Dynamic);
        let b_dynamic = body_b
            .and_then(|id| self.body_state(id).ok())
            .is_some_and(|body| body.desc.kind == BodyKind::Dynamic);
        match (body_a, body_b, a_dynamic, b_dynamic) {
            (Some(a), Some(b), true, true) => {
                self.move_body_by(a, -normal * (penetration * 0.5));
                self.move_body_by(b, normal * (penetration * 0.5));
                self.remove_velocity_into_normal(a, normal, restitution);
                self.remove_velocity_into_normal(b, -normal, restitution);
            }
            (Some(a), _, true, _) => {
                self.move_body_by(a, -normal * penetration);
                self.remove_velocity_into_normal(a, normal, restitution);
            }
            (_, Some(b), _, true) => {
                self.move_body_by(b, normal * penetration);
                self.remove_velocity_into_normal(b, -normal, restitution);
            }
            _ => {}
        }
    }

    fn move_body_by(&mut self, body: BodyId, mut delta: Vec3) {
        if let Ok(state) = self.body_state_mut(body) {
            apply_locked_translation(&mut delta, state.desc.lock_axes);
            state.transform.translation += delta;
            state.desc.transform = state.transform;
        }
    }

    fn remove_velocity_into_normal(&mut self, body: BodyId, normal: Vec3, restitution: Real) {
        let sleeping_linear_threshold = self.config.sleeping.linear_threshold;
        if let Ok(state) = self.body_state_mut(body) {
            let into = state.velocity.linear.dot(normal);
            if into > 0.0 {
                state.velocity.linear -= normal * into * (1.0 + restitution);
                if state.velocity.linear.length() < sleeping_linear_threshold {
                    state.velocity.linear = Vec3::ZERO;
                }
            }
        }
    }

    fn apply_joint_correction(&mut self, body_a: BodyId, body_b: BodyId, correction: Vec3) {
        let a_dynamic = self
            .body_state(body_a)
            .is_ok_and(|body| body.desc.kind == BodyKind::Dynamic);
        let b_dynamic = self
            .body_state(body_b)
            .is_ok_and(|body| body.desc.kind == BodyKind::Dynamic);
        match (a_dynamic, b_dynamic) {
            (true, true) => {
                self.move_body_by(body_a, correction * 0.5);
                self.move_body_by(body_b, -correction * 0.5);
            }
            (true, false) => self.move_body_by(body_a, correction),
            (false, true) => self.move_body_by(body_b, -correction),
            (false, false) => {}
        }
    }

    fn apply_joint_motor(&mut self, joint: &JointState) {
        match &joint.desc {
            JointDesc::Hinge(desc) => {
                if let Some(motor) = desc.motor {
                    self.apply_motor_velocity(joint.body_b, desc.anchors.local_axis_b, motor, true);
                }
            }
            JointDesc::Prismatic(desc) => {
                if let Some(motor) = desc.motor {
                    self.apply_motor_velocity(
                        joint.body_b,
                        desc.anchors.local_axis_b,
                        motor,
                        false,
                    );
                }
            }
            JointDesc::Generic(desc) => {
                for axis_motor in &desc.motors {
                    let axis = match axis_motor.axis {
                        JointAxis::X => Some((Vec3::X, false)),
                        JointAxis::Y => Some((Vec3::Y, false)),
                        JointAxis::Z => Some((Vec3::Z, false)),
                        JointAxis::AngularX => Some((Vec3::X, true)),
                        JointAxis::AngularY => Some((Vec3::Y, true)),
                        JointAxis::AngularZ => Some((Vec3::Z, true)),
                    };
                    if let Some((axis, angular)) = axis {
                        self.apply_motor_velocity(joint.body_b, axis, axis_motor.motor, angular);
                    }
                }
            }
            _ => {}
        }
    }

    fn apply_motor_velocity(&mut self, body: BodyId, axis: Vec3, motor: JointMotor, angular: bool) {
        let Ok(state) = self.body_state_mut(body) else {
            return;
        };
        if state.desc.kind != BodyKind::Dynamic {
            return;
        }
        let axis = axis.normalize_or_zero();
        if axis == Vec3::ZERO {
            return;
        }
        let target = axis * motor.target_velocity;
        if angular {
            state.velocity.angular += target;
        } else {
            state.velocity.linear += target;
        }
        state.sleeping = false;
    }

    fn parent_body_enabled(&self, body: Option<BodyId>) -> bool {
        body.and_then(|id| self.body_state(id).ok())
            .is_none_or(|state| state.desc.enabled)
    }

    fn collider_world_transform_internal(&self, collider: ColliderId) -> PhysicsResult<Transform> {
        let collider_state = self.collider_state(collider)?;
        if let Some(parent) = collider_state.parent {
            let parent = self.body_state(parent)?;
            Ok(Transform::compose(
                parent.transform,
                collider_state.desc.local_transform,
            ))
        } else {
            Ok(collider_state.desc.local_transform)
        }
    }

    fn validate_shape(&self, shape: &ColliderShape) -> PhysicsResult<()> {
        validate_shape_basic(shape)?;
        match shape {
            ColliderShape::ConvexHull { mesh }
            | ColliderShape::TriMesh { mesh, .. }
            | ColliderShape::HeightField { mesh } => {
                self.mesh_state(*mesh)?;
            }
            ColliderShape::Compound { parts } => {
                for part in parts {
                    validate_transform(part.local_transform)?;
                    self.validate_shape(&part.shape)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn shape_aabb(&self, shape: &ColliderShape, transform: Transform) -> Option<Aabb> {
        shape_aabb(shape, transform, |mesh| {
            self.mesh_state(mesh).ok().map(|m| &m.desc)
        })
    }

    fn debug_draw_shape(
        &self,
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
            ColliderShape::Cylinder {
                half_height,
                radius,
                ..
            }
            | ColliderShape::Cone {
                half_height,
                radius,
                ..
            } => {
                renderer.capsule(transform, Axis3::Y, *half_height, *radius, style);
            }
            ColliderShape::Compound { parts } => {
                for part in parts {
                    self.debug_draw_shape(
                        renderer,
                        &part.shape,
                        Transform::compose(transform, part.local_transform),
                        category,
                    );
                }
            }
            _ => {
                if let Some(aabb) = self.shape_aabb(shape, transform) {
                    renderer.cuboid(
                        Transform::from_translation(aabb.center()),
                        aabb.half_extents(),
                        style,
                    );
                }
            }
        }
    }

    fn query_filter_matches(
        &self,
        collider: ColliderId,
        state: &ColliderState,
        filter: QueryFilter,
    ) -> bool {
        if filter.exclude_collider == Some(collider)
            || filter.exclude_body.is_some() && filter.exclude_body == state.parent
            || (!filter.include_sensors && state.desc.sensor)
            || !state.desc.enabled
            || !state.desc.filter.query_matches(filter)
        {
            return false;
        }
        let Some(parent) = state.parent else {
            return filter.include_fixed;
        };
        let Ok(body) = self.body_state(parent) else {
            return false;
        };
        if !body.desc.enabled {
            return false;
        }
        match body.desc.kind {
            BodyKind::Dynamic => filter.include_dynamic,
            BodyKind::Fixed => filter.include_fixed,
            BodyKind::KinematicPosition | BodyKind::KinematicVelocity => filter.include_kinematic,
        }
    }

    fn record_query_gizmo(&self, gizmo: QueryGizmo) {
        if self.config.debug.enabled && self.config.debug.record_query_gizmos {
            self.query_gizmos.borrow_mut().push(gizmo);
        }
    }

    fn recompute_body_mass(&mut self, body: BodyId) {
        let Ok(state) = self.body_state(body) else {
            return;
        };
        if !matches!(state.desc.mass, MassDesc::Auto) {
            return;
        }
        let mut mass = 0.0;
        for (_, collider) in self.colliders_iter() {
            if collider.parent == Some(body) && !collider.desc.sensor {
                mass += self.shape_volume(&collider.desc.shape).unwrap_or(1.0)
                    * collider.desc.density.max(0.0);
            }
        }
        if let Ok(state) = self.body_state_mut(body) {
            state.mass = mass.max(1.0);
        }
    }

    fn shape_volume(&self, shape: &ColliderShape) -> Option<Real> {
        shape_aabb(shape, Transform::IDENTITY, |mesh| {
            self.mesh_state(mesh).ok().map(|m| &m.desc)
        })
        .map(|aabb| {
            let size = aabb.max - aabb.min;
            (size.x.abs() * size.y.abs() * size.z.abs()).max(0.0001)
        })
    }

    fn validate_snapshot(&self, snapshot: &PhysicsSnapshot) -> PhysicsResult<()> {
        let mut bodies = BTreeSet::new();
        let mut colliders = BTreeSet::new();
        let mut joints = BTreeSet::new();
        let mut meshes = BTreeSet::new();
        for mesh in &snapshot.meshes {
            if !mesh.id.is_valid() || !meshes.insert(mesh.id.raw()) {
                return Err(PhysicsError::InvalidSnapshot(
                    "duplicate or invalid mesh id".to_owned(),
                ));
            }
        }
        for body in &snapshot.bodies {
            if !body.id.is_valid() || !bodies.insert(body.id.raw()) {
                return Err(PhysicsError::InvalidSnapshot(
                    "duplicate or invalid body id".to_owned(),
                ));
            }
            validate_transform(body.transform)?;
            validate_transform(body.previous_transform)?;
        }
        for collider in &snapshot.colliders {
            if !collider.id.is_valid() || !colliders.insert(collider.id.raw()) {
                return Err(PhysicsError::InvalidSnapshot(
                    "duplicate or invalid collider id".to_owned(),
                ));
            }
            if let Some(parent) = collider.parent {
                if !bodies.contains(&parent.raw()) {
                    return Err(PhysicsError::InvalidParent(parent));
                }
            }
            validate_shape_basic(&collider.desc.shape)?;
            validate_transform(collider.desc.local_transform)?;
        }
        for joint in &snapshot.joints {
            if !joint.id.is_valid() || !joints.insert(joint.id.raw()) {
                return Err(PhysicsError::InvalidSnapshot(
                    "duplicate or invalid joint id".to_owned(),
                ));
            }
            if !bodies.contains(&joint.body_a.raw()) {
                return Err(PhysicsError::BodyNotFound(joint.body_a));
            }
            if !bodies.contains(&joint.body_b.raw()) {
                return Err(PhysicsError::BodyNotFound(joint.body_b));
            }
        }
        Ok(())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DestroyedObjects {
    pub bodies: Vec<BodyId>,
    pub colliders: Vec<ColliderId>,
    pub joints: Vec<JointId>,
}

macro_rules! slot_accessors {
    (
        $alloc_fn:ident, $release_fn:ident, $insert_fn:ident,
        $get_fn:ident, $get_mut_fn:ident, $iter_fn:ident, $iter_mut_fn:ident,
        $slots:ident, $free:ident, $id:ty, $state:ty, $not_found:expr
    ) => {
        fn $alloc_fn(&mut self, state: $state) -> $id {
            if let Some(index) = self.$free.pop() {
                let generation = self.$slots[index as usize].generation;
                self.$slots[index as usize].value = Some(state);
                <$id>::from_parts(index, generation)
            } else {
                let index = self.$slots.len() as u32;
                self.$slots.push(Slot::occupied(1, state));
                <$id>::from_parts(index, 1)
            }
        }

        fn $release_fn(&mut self, id: $id) -> PhysicsResult<()> {
            let index = id.index() as usize;
            if index >= self.$slots.len()
                || self.$slots[index].generation != id.generation()
                || self.$slots[index].value.is_none()
            {
                return Err($not_found(id));
            }
            self.$slots[index].value = None;
            self.$slots[index].generation = self.$slots[index].generation.wrapping_add(1).max(1);
            self.$free.push(index as u32);
            Ok(())
        }

        fn $insert_fn(&mut self, id: $id, state: $state) -> PhysicsResult<()> {
            if !id.is_valid() {
                return Err(PhysicsError::InvalidSnapshot(
                    "invalid id in snapshot".to_owned(),
                ));
            }
            let index = id.index() as usize;
            if self.$slots.len() <= index {
                self.$slots.resize_with(index + 1, || Slot {
                    generation: 1,
                    value: None,
                });
            }
            if self.$slots[index].value.is_some() {
                return Err(PhysicsError::AlreadyExists);
            }
            self.$slots[index].generation = id.generation().max(1);
            self.$slots[index].value = Some(state);
            self.$free.retain(|candidate| *candidate as usize != index);
            Ok(())
        }

        fn $get_fn(&self, id: $id) -> PhysicsResult<&$state> {
            let index = id.index() as usize;
            self.$slots
                .get(index)
                .filter(|slot| slot.generation == id.generation())
                .and_then(|slot| slot.value.as_ref())
                .ok_or_else(|| $not_found(id))
        }

        #[allow(dead_code)]
        fn $get_mut_fn(&mut self, id: $id) -> PhysicsResult<&mut $state> {
            let index = id.index() as usize;
            self.$slots
                .get_mut(index)
                .filter(|slot| slot.generation == id.generation())
                .and_then(|slot| slot.value.as_mut())
                .ok_or_else(|| $not_found(id))
        }

        fn $iter_fn(&self) -> impl Iterator<Item = ($id, &$state)> {
            self.$slots.iter().enumerate().filter_map(|(index, slot)| {
                slot.value
                    .as_ref()
                    .map(|state| (<$id>::from_parts(index as u32, slot.generation), state))
            })
        }

        #[allow(dead_code)]
        fn $iter_mut_fn(&mut self) -> impl Iterator<Item = ($id, &mut $state)> {
            self.$slots
                .iter_mut()
                .enumerate()
                .filter_map(|(index, slot)| {
                    let generation = slot.generation;
                    slot.value
                        .as_mut()
                        .map(|state| (<$id>::from_parts(index as u32, generation), state))
                })
        }
    };
}

impl PhysicsWorld {
    slot_accessors!(
        allocate_body,
        release_body,
        insert_body_with_id,
        body_state,
        body_state_mut,
        bodies_iter,
        bodies_iter_mut,
        bodies,
        free_bodies,
        BodyId,
        BodyState,
        PhysicsError::BodyNotFound
    );

    slot_accessors!(
        allocate_collider,
        release_collider,
        insert_collider_with_id,
        collider_state,
        collider_state_mut,
        colliders_iter,
        colliders_iter_mut,
        colliders,
        free_colliders,
        ColliderId,
        ColliderState,
        PhysicsError::ColliderNotFound
    );

    slot_accessors!(
        allocate_mesh,
        release_mesh,
        insert_mesh_with_id,
        mesh_state,
        mesh_state_mut,
        meshes_iter,
        meshes_iter_mut,
        meshes,
        free_meshes,
        PhysicsMeshId,
        MeshState,
        PhysicsError::MeshNotFound
    );

    slot_accessors!(
        allocate_joint,
        release_joint,
        insert_joint_with_id,
        joint_state,
        joint_state_mut,
        joints_iter,
        joints_iter_mut,
        joints,
        free_joints,
        JointId,
        JointState,
        PhysicsError::JointNotFound
    );

    slot_accessors!(
        allocate_character_controller,
        release_character_controller,
        insert_character_controller_with_id,
        character_controller_state,
        character_controller_state_mut,
        character_controllers_iter,
        character_controllers_iter_mut,
        character_controllers,
        free_character_controllers,
        CharacterControllerId,
        CharacterControllerState,
        PhysicsError::CharacterControllerNotFound
    );
}

fn mass_value(desc: MassDesc) -> Real {
    match desc {
        MassDesc::Auto => 1.0,
        MassDesc::Explicit { mass, .. } => mass.max(Real::EPSILON),
        MassDesc::Infinite => Real::INFINITY,
    }
}

fn validate_mass(mass: MassDesc) -> PhysicsResult<()> {
    match mass {
        MassDesc::Auto | MassDesc::Infinite => Ok(()),
        MassDesc::Explicit {
            mass,
            center_of_mass,
            principal_inertia,
        } => {
            if mass.is_finite()
                && mass > 0.0
                && center_of_mass.is_finite()
                && principal_inertia.is_finite()
            {
                Ok(())
            } else {
                Err(PhysicsError::InvalidShape {
                    reason: "explicit mass must be positive and finite".to_owned(),
                })
            }
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

fn validate_vec3(value: Vec3) -> PhysicsResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(PhysicsError::InvalidTransform)
    }
}

fn validate_shape_basic(shape: &ColliderShape) -> PhysicsResult<()> {
    match shape {
        ColliderShape::Sphere { radius } => validate_positive(*radius, "sphere radius"),
        ColliderShape::Cuboid { half_extents } => {
            if half_extents.is_finite()
                && half_extents.x > 0.0
                && half_extents.y > 0.0
                && half_extents.z > 0.0
            {
                Ok(())
            } else {
                Err(invalid_shape(
                    "cuboid half extents must be positive and finite",
                ))
            }
        }
        ColliderShape::Capsule {
            half_height,
            radius,
            ..
        }
        | ColliderShape::Cylinder {
            half_height,
            radius,
            ..
        }
        | ColliderShape::Cone {
            half_height,
            radius,
            ..
        } => {
            validate_positive(*half_height, "half height")?;
            validate_positive(*radius, "radius")
        }
        ColliderShape::Compound { parts } => {
            if parts.is_empty() {
                return Err(invalid_shape("compound shape requires at least one part"));
            }
            for CompoundShapePart {
                local_transform,
                shape,
            } in parts
            {
                validate_transform(*local_transform)?;
                validate_shape_basic(shape)?;
            }
            Ok(())
        }
        ColliderShape::ConvexHull { mesh }
        | ColliderShape::TriMesh { mesh, .. }
        | ColliderShape::HeightField { mesh } => {
            if mesh.is_valid() {
                Ok(())
            } else {
                Err(invalid_shape("mesh shape requires a valid mesh id"))
            }
        }
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

fn invalid_shape(reason: &str) -> PhysicsError {
    PhysicsError::InvalidShape {
        reason: reason.to_owned(),
    }
}

fn validate_trimesh(desc: &TriMeshDesc) -> PhysicsResult<()> {
    if desc.vertices.len() < 3 || desc.indices.is_empty() {
        return Err(invalid_shape("trimesh requires vertices and triangles"));
    }
    if !desc.vertices.iter().all(|v| v.is_finite()) {
        return Err(invalid_shape("trimesh vertices must be finite"));
    }
    let len = desc.vertices.len() as u32;
    if desc.indices.iter().flatten().any(|index| *index >= len) {
        return Err(invalid_shape("trimesh index out of range"));
    }
    Ok(())
}

fn validate_convex_mesh(desc: &ConvexMeshDesc) -> PhysicsResult<()> {
    if desc.points.len() < 4 || !desc.points.iter().all(|point| point.is_finite()) {
        return Err(invalid_shape(
            "convex mesh requires at least four finite points",
        ));
    }
    Ok(())
}

fn validate_heightfield(desc: &HeightFieldDesc) -> PhysicsResult<()> {
    if desc.rows < 2
        || desc.cols < 2
        || desc.heights.len() != (desc.rows * desc.cols) as usize
        || !desc.heights.iter().all(|height| height.is_finite())
        || !desc.scale.is_finite()
    {
        return Err(invalid_shape(
            "heightfield dimensions, heights, and scale must be valid",
        ));
    }
    Ok(())
}

fn shape_references_mesh(shape: &ColliderShape, mesh: PhysicsMeshId) -> bool {
    match shape {
        ColliderShape::ConvexHull { mesh: candidate }
        | ColliderShape::TriMesh {
            mesh: candidate, ..
        }
        | ColliderShape::HeightField { mesh: candidate } => *candidate == mesh,
        ColliderShape::Compound { parts } => parts
            .iter()
            .any(|part| shape_references_mesh(&part.shape, mesh)),
        _ => false,
    }
}

fn shape_aabb<'a>(
    shape: &ColliderShape,
    transform: Transform,
    mesh_lookup: impl Fn(PhysicsMeshId) -> Option<&'a PhysicsMeshDesc> + Copy,
) -> Option<Aabb> {
    let center = transform.translation;
    match shape {
        ColliderShape::Sphere { radius } => {
            Some(Aabb::from_center_half_extents(center, Vec3::splat(*radius)))
        }
        ColliderShape::Cuboid { half_extents } => Some(Aabb::from_center_half_extents(
            center,
            *half_extents * transform.scale.abs(),
        )),
        ColliderShape::Capsule {
            axis,
            half_height,
            radius,
        }
        | ColliderShape::Cylinder {
            axis,
            half_height,
            radius,
        }
        | ColliderShape::Cone {
            axis,
            half_height,
            radius,
        } => {
            let mut half = Vec3::splat(*radius);
            match axis {
                Axis3::X => half.x += *half_height,
                Axis3::Y => half.y += *half_height,
                Axis3::Z => half.z += *half_height,
            }
            Some(Aabb::from_center_half_extents(
                center,
                half * transform.scale.abs(),
            ))
        }
        ColliderShape::ConvexHull { mesh }
        | ColliderShape::TriMesh { mesh, .. }
        | ColliderShape::HeightField { mesh } => mesh_lookup(*mesh).and_then(|desc| {
            bounds_from_points(
                desc.points()
                    .into_iter()
                    .map(|point| center + point * transform.scale),
            )
        }),
        ColliderShape::Compound { parts } => {
            let mut result = None;
            for part in parts {
                let child = shape_aabb(
                    &part.shape,
                    Transform::compose(transform, part.local_transform),
                    mesh_lookup,
                )?;
                result = Some(result.map_or(child, |aabb: Aabb| aabb.union(child)));
            }
            result
        }
    }
}

fn bounds_from_points(points: impl Iterator<Item = Vec3>) -> Option<Aabb> {
    let mut iter = points.peekable();
    iter.peek()?;
    let mut min = Vec3::splat(Real::INFINITY);
    let mut max = Vec3::splat(Real::NEG_INFINITY);
    for point in iter {
        min = min.min(point);
        max = max.max(point);
    }
    Some(Aabb::new(min, max))
}

fn ray_aabb(origin: Vec3, direction: Vec3, max_toi: Real, aabb: Aabb) -> Option<Real> {
    let mut tmin: Real = 0.0;
    let mut tmax = max_toi;
    for (origin_axis, direction_axis, min_axis, max_axis) in [
        (origin.x, direction.x, aabb.min.x, aabb.max.x),
        (origin.y, direction.y, aabb.min.y, aabb.max.y),
        (origin.z, direction.z, aabb.min.z, aabb.max.z),
    ] {
        if direction_axis.abs() <= Real::EPSILON {
            if origin_axis < min_axis || origin_axis > max_axis {
                return None;
            }
        } else {
            let inv_d = 1.0 / direction_axis;
            let mut t1 = (min_axis - origin_axis) * inv_d;
            let mut t2 = (max_axis - origin_axis) * inv_d;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }
    }
    Some(tmin)
}

fn aabb_normal_at_point(aabb: Aabb, point: Vec3) -> Vec3 {
    let distances = [
        ((point.x - aabb.min.x).abs(), Vec3::new(-1.0, 0.0, 0.0)),
        ((point.x - aabb.max.x).abs(), Vec3::new(1.0, 0.0, 0.0)),
        ((point.y - aabb.min.y).abs(), Vec3::new(0.0, -1.0, 0.0)),
        ((point.y - aabb.max.y).abs(), Vec3::new(0.0, 1.0, 0.0)),
        ((point.z - aabb.min.z).abs(), Vec3::new(0.0, 0.0, -1.0)),
        ((point.z - aabb.max.z).abs(), Vec3::new(0.0, 0.0, 1.0)),
    ];
    distances
        .into_iter()
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, normal)| normal)
        .unwrap_or(Vec3::Y)
}

fn contact_manifold(
    a: ColliderId,
    b: ColliderId,
    body_a: Option<BodyId>,
    body_b: Option<BodyId>,
    aabb: Aabb,
    babb: Aabb,
) -> ContactManifold {
    let ac = aabb.center();
    let bc = babb.center();
    let delta = bc - ac;
    let overlap = aabb.half_extents() + babb.half_extents() - delta.abs();
    let (normal, penetration) = if overlap.x <= overlap.y && overlap.x <= overlap.z {
        (
            Vec3::new(delta.x.signum().max(-1.0).min(1.0), 0.0, 0.0),
            overlap.x,
        )
    } else if overlap.y <= overlap.z {
        (
            Vec3::new(0.0, if delta.y >= 0.0 { 1.0 } else { -1.0 }, 0.0),
            overlap.y,
        )
    } else {
        (
            Vec3::new(0.0, 0.0, if delta.z >= 0.0 { 1.0 } else { -1.0 }),
            overlap.z,
        )
    };
    let normal = if normal == Vec3::ZERO {
        Vec3::Y
    } else {
        normal
    };
    ContactManifold {
        a,
        b,
        body_a,
        body_b,
        contacts: vec![ContactPoint {
            position: (ac + bc) * 0.5,
            normal,
            penetration: penetration.max(0.0),
            impulse: penetration.max(0.0),
        }],
    }
}

fn active_collision_events(a: &ColliderDesc, b: &ColliderDesc) -> bool {
    a.events.contains(ActiveEvents::COLLISION_EVENTS)
        || b.events.contains(ActiveEvents::COLLISION_EVENTS)
}

fn active_force_events(a: &ColliderDesc, b: &ColliderDesc) -> bool {
    a.events.contains(ActiveEvents::CONTACT_FORCE_EVENTS)
        || b.events.contains(ActiveEvents::CONTACT_FORCE_EVENTS)
}

fn sensor_event(
    tick: PhysicsTick,
    a_id: ColliderId,
    b_id: ColliderId,
    a: &ColliderState,
    b: &ColliderState,
) -> Option<SensorEvent> {
    let (sensor, other, sensor_state, other_state) = if a.desc.sensor {
        (a_id, b_id, a, b)
    } else if b.desc.sensor {
        (b_id, a_id, b, a)
    } else {
        return None;
    };
    if !sensor_state
        .desc
        .events
        .contains(ActiveEvents::SENSOR_EVENTS)
        && !other_state
            .desc
            .events
            .contains(ActiveEvents::SENSOR_EVENTS)
    {
        return None;
    }
    Some(SensorEvent {
        tick,
        sensor,
        other,
        sensor_body: sensor_state.parent,
        other_body: other_state.parent,
    })
}

fn relative_velocity(world: &PhysicsWorld, a: Option<BodyId>, b: Option<BodyId>) -> Vec3 {
    let va = a
        .and_then(|id| world.body_state(id).ok())
        .map(|body| body.velocity.linear)
        .unwrap_or(Vec3::ZERO);
    let vb = b
        .and_then(|id| world.body_state(id).ok())
        .map(|body| body.velocity.linear)
        .unwrap_or(Vec3::ZERO);
    vb - va
}

fn mass_for_body(world: &PhysicsWorld, body: Option<BodyId>) -> Real {
    body.and_then(|id| world.body_state(id).ok())
        .map(|body| body.mass)
        .filter(|mass| mass.is_finite())
        .unwrap_or(1.0)
}

fn event_rank(event: &PhysicsEvent) -> u8 {
    match event {
        PhysicsEvent::CollisionStarted(_) => 0,
        PhysicsEvent::CollisionStopped(_) => 1,
        PhysicsEvent::SensorEntered(_) => 2,
        PhysicsEvent::SensorExited(_) => 3,
        PhysicsEvent::ContactForce(_) => 4,
        PhysicsEvent::EventDropped(_) => 5,
    }
}

fn update_sleeping(body: &mut BodyState, config: &PhysicsConfig, dt: Real) {
    if !config.sleeping.enabled || !body.desc.can_sleep {
        return;
    }
    if body.velocity.linear.length() <= config.sleeping.linear_threshold
        && body.velocity.angular.length() <= config.sleeping.angular_threshold
    {
        body.sleep_timer += dt;
        if body.sleep_timer >= config.sleeping.minimum_sleep_time {
            body.sleeping = true;
            body.velocity = Velocity::default();
        }
    } else {
        body.sleep_timer = 0.0;
        body.sleeping = false;
    }
}

fn apply_locked_translation(delta: &mut Vec3, locked: crate::body::LockedAxes) {
    if locked.contains(crate::body::LockedAxes::TRANSLATION_X) {
        delta.x = 0.0;
    }
    if locked.contains(crate::body::LockedAxes::TRANSLATION_Y) {
        delta.y = 0.0;
    }
    if locked.contains(crate::body::LockedAxes::TRANSLATION_Z) {
        delta.z = 0.0;
    }
}

fn apply_locked_angular_velocity(angular: &mut Vec3, locked: crate::body::LockedAxes) {
    if locked.contains(crate::body::LockedAxes::ROTATION_X) {
        angular.x = 0.0;
    }
    if locked.contains(crate::body::LockedAxes::ROTATION_Y) {
        angular.y = 0.0;
    }
    if locked.contains(crate::body::LockedAxes::ROTATION_Z) {
        angular.z = 0.0;
    }
}

fn integrate_rotation(transform: &mut Transform, angular_velocity: Vec3, dt: Real) {
    let angle = angular_velocity.length() * dt;
    if angle > Real::EPSILON {
        let delta = Quat::from_axis_angle(angular_velocity.normalize_or_zero(), angle);
        transform.rotation = delta.mul_quat(transform.rotation);
    }
}

fn draw_aabb(renderer: &mut dyn PhysicsDebugRenderer, aabb: Aabb, category: DebugDrawCategory) {
    renderer.cuboid(
        Transform::from_translation(aabb.center()),
        aabb.half_extents(),
        DebugShapeStyle::new(category),
    );
}

fn validate_joint_desc(desc: &JointDesc) -> PhysicsResult<()> {
    match desc {
        JointDesc::Distance(desc) => {
            if desc.min_distance.is_finite()
                && desc.max_distance.is_finite()
                && desc.min_distance >= 0.0
                && desc.max_distance >= desc.min_distance
            {
                Ok(())
            } else {
                Err(PhysicsError::InvalidShape {
                    reason: "distance joint requires 0 <= min <= max".to_owned(),
                })
            }
        }
        _ => Ok(()),
    }
}

fn validate_motor(motor: JointMotor) -> PhysicsResult<()> {
    if motor.target_velocity.is_finite()
        && motor.target_position.is_none_or(|value| value.is_finite())
        && motor.stiffness.is_finite()
        && motor.damping.is_finite()
        && motor.max_force.is_finite()
        && motor.max_force >= 0.0
    {
        Ok(())
    } else {
        Err(PhysicsError::InvalidShape {
            reason: "joint motor values must be finite".to_owned(),
        })
    }
}

fn validate_limits(limits: JointLimits) -> PhysicsResult<()> {
    if limits.min.is_finite() && limits.max.is_finite() && limits.min <= limits.max {
        Ok(())
    } else {
        Err(PhysicsError::InvalidShape {
            reason: "joint limits require min <= max".to_owned(),
        })
    }
}

fn set_joint_motor_on_desc(desc: &mut JointDesc, axis: JointAxis, motor: JointMotor) {
    match desc {
        JointDesc::Hinge(desc) => desc.motor = Some(motor),
        JointDesc::Prismatic(desc) => desc.motor = Some(motor),
        JointDesc::Generic(desc) => {
            if let Some(existing) = desc
                .motors
                .iter_mut()
                .find(|existing| existing.axis == axis)
            {
                existing.motor = motor;
            } else {
                desc.motors
                    .push(crate::joint::JointAxisMotor { axis, motor });
            }
        }
        _ => {}
    }
}

fn set_joint_limits_on_desc(desc: &mut JointDesc, axis: JointAxis, limits: JointLimits) {
    match desc {
        JointDesc::Ball(desc) => desc.limits = Some(limits),
        JointDesc::Hinge(desc) => desc.limits = Some(limits),
        JointDesc::Prismatic(desc) => desc.limits = Some(limits),
        JointDesc::Generic(desc) => {
            if let Some(existing) = desc
                .limits
                .iter_mut()
                .find(|existing| existing.axis == axis)
            {
                existing.min = limits.min;
                existing.max = limits.max;
            } else {
                desc.limits.push(crate::joint::JointAxisLimit {
                    axis,
                    min: limits.min,
                    max: limits.max,
                });
            }
        }
        _ => {}
    }
}

fn joint_distance_limits(desc: &JointDesc, a: Transform, b: Transform) -> (Real, Real) {
    match desc {
        JointDesc::Fixed(_) => (0.0, 0.0),
        JointDesc::Distance(desc) => (desc.min_distance, desc.max_distance),
        JointDesc::Ball(desc) => desc
            .limits
            .map(|limits| (limits.min, limits.max))
            .unwrap_or((0.0, a.translation.distance(b.translation))),
        JointDesc::Hinge(desc) => desc
            .limits
            .map(|limits| (limits.min, limits.max))
            .unwrap_or((0.0, a.translation.distance(b.translation))),
        JointDesc::Prismatic(desc) => desc
            .limits
            .map(|limits| (limits.min, limits.max))
            .unwrap_or((0.0, Real::INFINITY)),
        JointDesc::Generic(_) => (0.0, a.translation.distance(b.translation)),
    }
}
