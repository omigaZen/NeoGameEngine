# Codex Goal Prompt: Implement NeoGameEngine Physics Module

## Objective

Implement the complete NeoGameEngine physics module described by `docs/engine_physics_api.md`.

The finished module must let game, ECS, scene, and tooling code use physics only through a stable `engine_physics` public API. It must not expose Rapier or any other backend-specific type through the public facade.

This goal is complete only when the repository contains a production-usable physics layer with real object lifecycle, fixed-step simulation, collision/events, queries, character movement, joints, debug draw, snapshot/restore, ECS integration, backend abstraction, focused tests, examples, and synchronized documentation.

## Sources Of Truth

- Primary API contract: `docs/engine_physics_api.md`
- Goal prompt: `docs/physics_goal.md`
- Required implementation queue: `docs/physics_goal_coverage_matrix.md`
- Required audit notes: `docs/physics_goal_audit_report.md`

If the coverage matrix or audit report does not exist yet, create it before claiming any implementation slice complete.

## Repository Context

- Workspace root: `C:\Playground\NeoGameEngine`
- New public crate should be named `engine_physics` unless existing workspace structure clearly requires another crate name.
- Preferred backend is Rapier through a `backend_rapier` feature.
- Public API must remain backend-agnostic.
- Physics should use the engine-facing math/types defined or chosen by the module, initially allowed to use `glam` as described in the API document.

## Product Scope

Build a complete physics product layer, not a demo or API shell. The completed module must cover:

- Module structure and `prelude`.
- Core types: `Real`, vectors/quaternions, `Transform`, stable IDs, `PhysicsUserData`, `PhysicsError`, `PhysicsResult`.
- `PhysicsWorld` lifecycle, config access, gravity, tick/frame index, fixed stepping, accumulator, interpolation alpha.
- Config: timestep, solver, sleeping, CCD, events, debug, determinism.
- Body API: body kinds, descriptors/builders, create/destroy/recursive destroy, status queries, transform, previous/interpolated transform, velocity, mass, enabled/sleeping, user data, teleport, kinematic targets, forces, impulses, torques, wake/sleep.
- Collider API: primitive shapes, mesh-backed shapes, compound shapes, material, density, filters, sensors, active events, enabled state, parent body relations, create/destroy/query/update.
- Mesh resources: trimesh, convex mesh, heightfield, lifecycle and stale/destroyed/missing errors.
- Materials and filters: friction, restitution, combine rules, collision groups/masks, query filters, sensor inclusion, body/collider exclusions.
- Command buffer: collect/apply gameplay physics commands with deterministic ordering and command application reporting.
- Events: collision start/stop, sensor enter/exit, contact force, contact data, event cache/drain, stable ordering, max event behavior.
- Queries: raycast, shapecast, overlap, hit payloads, filters, out-vector/batch behavior where documented, query debug gizmos.
- Contacts: contact point, manifold, pair query, body/collider contact listing.
- Character controller: controller lifecycle, compute-only movement, move-and-writeback, slide, auto-step, snap-to-ground, grounded/wall/ceiling output, dynamic-body interaction where supported.
- Joints: fixed, ball, hinge, prismatic, distance, generic, limits, motors, enabled state, body binding, debug draw, error paths.
- Debug draw: bodies, colliders, AABBs, contacts, normals, joints, sleeping state, query gizmos, names, and integration points for renderer debug output.
- Snapshot/restore: config, tick, bodies, colliders, joints, previous transforms, velocity, sleeping, stable IDs, restore validation.
- ECS integration: components, sync modes, pre/post physics sync, transform ownership rules, recommended system ordering.
- Backend abstraction: `PhysicsBackend`, `PhysicsQueryBackend`, Rapier adapter, backend capability reporting, backend error mapping.
- Hooks: collision pair filtering, contact modification, custom gameplay collision rules.
- Feature flags: `3d`, `2d`, `backend_rapier`, `serde`, `debug_draw`, `parallel`, `deterministic`.
- Examples: fixed ground, falling dynamic body, fixed timestep loop, transform writeback, raycast, collision events, debug draw, character movement, snapshot/restore, ECS sync, backend smoke.

## Non-Goals

Do not make the physics module responsible for gameplay outcomes or unrelated systems:

- Damage, health, AI decisions, animation state machines, audio playback, particles, quests, networking protocol, or renderer hierarchy ownership.
- Direct public dependency on Rapier, PhysX, Bullet, or any backend type.
- Declaring completion from mock-only behavior, fixed reports, label-only debug output, or facade-only type definitions.

## Execution Rules

Follow this loop until the objective is genuinely complete:

1. Read `docs/engine_physics_api.md` and the current implementation before editing.
2. Update or create `docs/physics_goal_coverage_matrix.md` so the next work item is explicit.
3. Pick a repository-implementable `Missing`, `Stub`, or `Partial` item from the matrix.
4. Implement the smallest coherent slice that closes public API, backend/local execution, error behavior, observability, tests, and docs together.
5. Add or update focused tests for success, failure, lifecycle, unsupported/capability-gated behavior, and observability.
6. Run the narrow relevant tests. Broaden test scope when touching shared APIs, backend behavior, ECS sync, snapshot, or workspace integration.
7. Update the coverage matrix and audit report with the exact evidence, remaining scope, and test commands.
8. Continue to the next uncovered item instead of declaring completion early.

Every new public API, feature flag, report/debug/snapshot field, example behavior, test helper, or backend API must be added to the coverage matrix in the same slice.

## Status Rules

Use these exact meanings in the coverage matrix:

- `Implemented`: public API is reachable, real execution exists, relevant backend or deterministic local path exists, errors are user-visible, observability exists, focused tests pass, and docs are synchronized.
- `Partial`: meaningful behavior exists, but a required public API, backend path, error path, observability surface, test, example, or documentation link is still incomplete.
- `Stub`: API or shape exists but behavior is placeholder, label-only, mock-only, fixed-report-only, unsupported-only, or helper-only.
- `Missing`: the documented capability is absent.
- `External Blocked`: only for true repository-external limits such as backend SDK absence, platform-native backend absence, backend-unexposed solver internals, or numeric/platform behavior that cannot be stably simulated. It still requires a capability gate, user-visible error, tests, matrix explanation, and follow-up entry point.

Do not mark a capability `Implemented` if the only proof is a type definition, mock, helper, fixed statistic, debug label, or documentation promise.

## Milestones

Milestone 1: Foundation and MVP loop

- Create the `engine_physics` crate/module structure, core types, errors, IDs, config, and prelude.
- Implement `PhysicsWorld`, fixed timestep, accumulator, step/frame reports, body/collider lifecycle, fixed ground, dynamic sphere/cube, gravity, velocity, transform writeback, force/impulse, kinematic target, basic destroy paths.
- Implement a minimal Rapier backend path if feasible; otherwise record the precise blocker and provide deterministic local behavior only where the API document allows it.
- Implement collision start/stop, sensor enter/exit, event drain, raycast, overlap, query filters, and debug draw for basic shapes.
- Add focused tests and a smoke example.

Milestone 2: Product baseline

- Add mesh resources, convex/trimesh/heightfield shapes, compound shapes, material combine rules, collision groups/masks, active events, contact force, contact manifolds, command buffer, ECS components and sync, interpolation, snapshot/restore, serde gate, and deterministic ordering tests.

Milestone 3: Advanced module completion

- Add shapecast, character controller, joints, motors/limits, hooks, contact modification, backend capability audit, feature gates, 2D/3D split behavior, debug/editor reports, renderer debug draw integration, backend parity tests, expanded examples, audit report, and final acceptance notes.

Milestones are execution order hints, not separate completion goals. The goal remains open until the full module is complete.

## Required Tests

For each completed slice, add or update focused tests for the relevant subset:

- Public API success path produces real physics state changes.
- Missing, destroyed, stale generation, invalid transform, invalid shape, invalid parent, unsupported, and backend error paths.
- Create, query, mutate, destroy, recursive destroy, ID reuse, and generation mismatch.
- `step_fixed`, `update_fixed`, accumulator clamp, max substeps, dropped steps, interpolation alpha.
- Rapier or current real backend execution path.
- Collision, sensor, contact force, event order, event drain, and max event behavior.
- Raycast, shapecast, overlap, filters, sensor include/exclude, exclude body/collider.
- Step/frame reports, debug draw output, snapshot/restore, ECS sync, and query gizmos.
- Feature-gated supported and unsupported behavior.

Before final completion, run and pass:

- `cargo test -p engine_physics`
- Relevant workspace tests for crates touched by integration work.
- Example builds or smoke runs for physics examples.
- Renderer integration tests if physics debug draw is wired into renderer debug output.

## Acceptance Criteria

The goal can be marked complete only when all of the following are true:

- `docs/physics_goal_coverage_matrix.md` covers every capability in `docs/engine_physics_api.md` plus all public APIs, examples, tests, backend APIs, debug/report/snapshot outputs, and implementation-added semantics.
- No matrix item remains `Missing` or `Stub`.
- Any remaining `Partial` is a true `External Blocked` item with capability gate, user-visible error, tests, explanation, and follow-up entry point.
- Users can build gameplay, ECS, scene sync, debug tooling, and examples using only `engine_physics` public APIs.
- Public API does not expose backend-specific types.
- Physics simulation uses fixed-step semantics, not raw render-frame delta stepping.
- Dynamic, fixed, kinematic-position, and kinematic-velocity ownership rules are implemented and tested.
- Object lifecycle includes stable IDs, generation/stale handling, destroyed/missing errors, recursive dependency handling, and tests.
- Collision, sensor, contact, query, character controller, joint, debug draw, snapshot/restore, ECS sync, hooks, and backend paths have real behavior or valid external-blocked entries.
- API docs, coverage matrix, audit report, implementation, tests, and examples all describe the same facts.

## Stop Conditions

Do not claim completion while any of these are true:

- A repository-implementable API from `docs/engine_physics_api.md` is missing, stubbed, mock-only, unsupported-only, or backend-unwired.
- A public feature is marked supported without a real execution path and tests.
- A public feature is marked unsupported only because it has not been implemented yet.
- Events, queries, debug draw, reports, snapshot, ECS sync, or backend errors return fake, empty, or label-only data where real data is implementable.
- Gameplay, ECS, or renderer integration must use Rapier or another backend directly.
- The coverage matrix, audit report, API doc, code, tests, or examples disagree.

## Final Response Format

When the objective is complete, report:

1. Summary of the implemented physics module.
2. Core files changed.
3. Tests and examples added or updated.
4. Exact test commands run and results.
5. Final status table for `Implemented`, `Partial`, `Stub`, `Missing`, and `External Blocked`.
6. Remaining external blockers, if any, with gates and follow-up entry points.
7. Confirmation that `docs/engine_physics_api.md`, `docs/physics_goal_coverage_matrix.md`, `docs/physics_goal_audit_report.md`, code, tests, and examples are synchronized.
