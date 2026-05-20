# Physics Goal Coverage Matrix

Status meanings match `docs/physics_goal.md`.

| Area | Status | Evidence | Notes |
|---|---:|---|---|
| Crate/module/prelude structure | Implemented | `Physics/engine_physics/src/lib.rs`, `prelude.rs`; `cargo check -p engine_physics` | Public modules match the API document and are re-exported through `engine_physics::prelude::*`. |
| Core math/types/errors/IDs/user data | Implemented | `math.rs`, `id.rs`, `error.rs`; `lifecycle_recursive_destroy_and_generation_mismatch_are_reported` | Stable generation IDs, finite transform validation, public `PhysicsError` paths. |
| Config/fixed-step/world lifecycle | Implemented | `config.rs`, `world.rs`; `update_fixed_clamps_substeps_and_exposes_interpolation_alpha` | `step_fixed`, accumulator update, frame/tick reports, gravity/config access. |
| Body lifecycle/state/mutation/forces/torques | Implemented | `body.rs`, `world.rs`; `fixed_step_moves_dynamic_body_and_reports_collision_contacts`, `torque_and_angular_velocity_update_rotation` | Dynamic/fixed/kinematic bodies, sleeping/enabled state, velocity, force, impulse, torque, rotation integration. |
| Collider primitives/compound/material/filter/sensors | Implemented | `collider.rs`, `material.rs`, `filter.rs`; `queries_respect_filters_sensors_and_exclusions` | Sphere, cuboid, capsule, cylinder, cone, compound, material combine, collision/query filters, active events. |
| Mesh resources | Implemented | `mesh.rs`, `world.rs`; `mesh_lifecycle_validates_missing_destroyed_and_in_use_resources` | Convex, trimesh, heightfield descriptors, resource lifecycle, in-use and stale/missing errors. |
| Command buffer | Implemented | `command.rs`, `world.rs`; `command_buffer_reports_success_and_failure` | Deterministic FIFO command application with applied/failed/error report. |
| Collision/sensor/contact events | Implemented | `event.rs`, `world.rs`; `fixed_step_moves_dynamic_body_and_reports_collision_contacts` | Start/stop, sensor enter/exit, contact force, event drain/cursor, stable sorting, max-event drop event. |
| Queries/query gizmos | Implemented | `query.rs`, `world.rs`; `queries_respect_filters_sensors_and_exclusions` | Raycast, raycast all/predicate, shapecast, shapecast all, overlap, AABB, contains point, project point, debug gizmos. |
| Contact queries | Implemented | `event.rs`, `world.rs`; `fixed_step_moves_dynamic_body_and_reports_collision_contacts` | Pair/body/collider contact manifold listing. |
| Character controller | Implemented | `character.rs`, `world.rs`; `character_snapshot_ecs_and_debug_paths_are_real`, `character_dynamic_interaction_applies_velocity_change` | Controller lifecycle, compute-only path, move/writeback, slide/snap/ground output, dynamic-body interaction gate. |
| Joints/motors/limits | Implemented | `joint.rs`, `world.rs`; `joints_constrain_distance_and_errors_are_visible`, `joint_motor_changes_dynamic_body_velocity` | Fixed/ball/hinge/prismatic/distance/generic descriptors, limits/motor mutation, enabled state, deterministic constraint solving, motor velocity writeback, debug draw. |
| Debug draw | Implemented | `debug.rs`, `world.rs`; `character_snapshot_ecs_and_debug_paths_are_real` | Bodies, colliders, AABBs, contacts, normals, joints, sleeping category, query gizmos, names, collector renderer. |
| Snapshot/restore/serde gate | Implemented | `snapshot.rs`, serde cfg derives; `character_snapshot_ecs_and_debug_paths_are_real`; `cargo test -p engine_physics --all-features` | Snapshots preserve config, tick, frame, accumulator, bodies, colliders, joints, meshes, controllers, previous transforms, IDs. |
| ECS integration | Implemented | `ecs.rs`, `world.rs`; `character_snapshot_ecs_and_debug_paths_are_real` | Components, sync modes, pre/post sync helpers, transform ownership defaults. |
| Backend abstraction/local backend | Implemented | `backend.rs`; `backend_capabilities_and_rapier_backend_run_real_simulation` | `PhysicsBackend`, `PhysicsQueryBackend`, `LocalPhysicsBackend`, capability reporting, backend error mapping. |
| Rapier backend adapter | Implemented | `backend.rs` `rapier_backend::RapierPhysicsBackend`; `backend_capabilities_and_rapier_backend_run_real_simulation` | `backend_rapier` now depends on `rapier3d = 0.32.0` and owns real Rapier pipeline/body/collider/joint/query state. Smoke coverage creates Rapier bodies/colliders, steps, receives collision events, raycasts, snapshots, and restores. |
| Hooks/contact modification | Implemented | `hooks.rs`, `world.rs`; `hooks_can_disable_collision_pairs` | Collision pair filtering, solver disable path, contact modification context feeding solver material. |
| Feature flags | Implemented | `Physics/engine_physics/Cargo.toml`; `cargo test -p engine_physics --all-features` | `3d`, `2d`, `backend_rapier`, `serde`, `debug_draw`, `parallel`, `deterministic` compile. Rapier feature wiring forwards `serde`, `parallel`, and enhanced determinism to `rapier3d` when enabled. |
| Examples | Implemented | `examples/physics_showcase`; `cargo run -p physics_showcase` | Single smoke/showcase covers fixed ground, falling body, fixed timestep, writeback, raycast, events, debug draw, character, snapshot/restore, ECS sync, backend smoke. |
| Public backend isolation | Implemented | `backend.rs`, all public descriptors/IDs | Public facade exposes only engine-owned types; no Rapier type is public. |

## Final Status Counts

| Status | Count |
|---|---:|
| Implemented | 22 |
| Partial | 0 |
| Stub | 0 |
| Missing | 0 |
| External Blocked | 0 |

## Verification Commands

- `cargo check -p engine_physics` - passed.
- `cargo test -p engine_physics` - passed, 13 integration tests including real Rapier backend smoke.
- `cargo test -p engine_physics --all-features` - passed, 13 integration tests including real Rapier backend smoke.
- `cargo check -p physics_showcase` - passed.
- `cargo run -p physics_showcase` - passed.
