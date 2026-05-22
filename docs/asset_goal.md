# Codex Goal Prompt: Implement NeoGameEngine Asset Management Module

## Objective

Implement the complete NeoGameEngine asset management module described by `docs/engine_asset_api.md`.

The finished module must let runtime systems, editor tooling, import/cook pipelines, bundles, streaming, rendering, audio, scenes, animation, physics, and ECS integration use assets through a stable `engine_asset` public API. It must not collapse into a path reader, a typed map, or a loader demo.

This goal is complete only when the repository contains a production-usable asset layer with stable asset identity, type-safe handles, typed storage, async loading state, dependency tracking, events, hot reload, GPU upload handoff, garbage collection, editor import, cooking, bundles, streaming, ECS integration, built-in asset types, focused tests, examples, and synchronized documentation.

## Sources Of Truth

- Primary API contract: `docs/engine_asset_api.md`
- Goal prompt: `docs/asset_goal.md`
- Required implementation queue: `docs/asset_goal_coverage_matrix.md`
- Required audit notes: `docs/asset_goal_audit_report.md`

If the coverage matrix or audit report does not exist yet, create it before claiming any implementation slice complete.

## Repository Context

- Workspace root: `C:\Playground\NeoGameEngine`
- New public crate should be named `engine_asset` unless existing workspace structure clearly requires another crate name.
- Asset identity must be based on stable `AssetId`, not mutable paths.
- Game/runtime code should hold `Handle<T>` or `UntypedHandle`, not raw loaded resource ownership.
- Runtime loading and editor importing must remain separate concepts.
- CPU loading and GPU upload must remain separate phases with a renderer-facing handoff.

## Product Scope

Build a complete asset product layer, not a file IO helper. The completed module must cover:

- Module structure and `prelude`.
- Core types: `AssetId`, `AssetTypeId`, `AssetTypeName`, `AssetPath`, `AssetKey`, `ContentHash`, `VersionHash`.
- `Asset` trait plus optional memory usage and dependency visitor traits.
- Handle system: `HandleStrength`, `Handle<T>`, `UntypedHandle`, `AssetRef<T>`, strong/weak semantics, typed/untyped conversion, serialization-facing references.
- Load states: `Unloaded`, `Queued`, `LoadingBytes`, `DecodingCpu`, `WaitingForDependencies`, `LoadedCpu`, `UploadingGpu`, `Ready`, `Failed`, `Reloading`, `Unloading`.
- Events: request/start/CPU loaded/ready/failed/reload/unload/dependency/GPU upload events, event cursors, draining, and event retention rules.
- Typed storage: `AssetEntry<T>`, `Assets<T>`, state/error tracking, strong/weak/dependency counts, resident flags, last-used frame, iteration, mutation, and mark-used behavior.
- `AssetServer`: config, registry, IO, loader registration, type registration, load/preload/load-by-id/load group APIs, access APIs, update phases, reload, unload, metadata lookup, path/id mapping, events, GPU upload handoff, hot reload, and GC.
- Load scheduling: `LoadPriority`, `LoadRequest`, queue ordering, cancellation, deduplication, progress reporting, and group state.
- IO layer: `AssetIo`, filesystem IO, bundle IO, composite IO, memory or test IO where useful, metadata, ranges, listing, and IO error mapping.
- Loader system: `AssetLoader`, `LoaderSettings`, `LoadContext`, `LoadedAsset`, loader registry, extension/type matching, dependency registration, subresource registration, and decode errors.
- Importer system: `AssetImporter`, `SourceAsset`, `ImporterSettings`, `ImportContext`, `ImportOutput`, importer registry, source hash/settings hash/version hash, generated artifacts, subresources, and import errors.
- Metadata and registry: `.meta` data, source/cooked paths, importer/cooker versions, hashes, labels, dependencies, asset registry lookup by id/path, save/load, scan, and rename/path fallback behavior.
- Dependency graph: graph construction, direct/transitive dependencies, reverse dependencies, topological order, cycle detection, dependency failure propagation, dependency ref counts.
- Cooker system: target platform config, cook context, cook outputs, platform-specific cooked bytes, content hash/version hash, and cook errors.
- Bundle system: bundle manifest, asset entries, compression, dependencies, mount/unmount, preload bundle, bundle reader/writer/builder, runtime bundle registry, and Bundle IO.
- Hot reload: file watching or explicit reload path, reload state, dependency reload behavior, handle stability, failed reload rollback/error behavior.
- GPU upload queue: upload commands, GPU handles, texture/mesh/material/shader upload metadata, upload results, per-frame limits, renderer handoff, failure paths, and upload events.
- GC and memory budget: strong/weak/dependency references, resident assets, unused unload, memory info, per-type stats, CPU/GPU byte accounting, budget-driven eviction, and unload events.
- Streaming: streaming regions, priorities, preload/unload by region, group progress, bundle/region integration, and residency interaction.
- `AssetDatabase`: editor-side source scanning, importer registration, import by path, cook by id, registry save/load, metadata access, dependency graph, bundle build integration.
- Built-in asset types: texture, mesh, model data where applicable, material, shader, audio clip, animation clip, skeleton, scene asset, prefab, font, physics mesh.
- Built-in loaders/importers: texture, mesh, material, shader, audio, model importer, texture importer, and dependency-producing material/model paths.
- ECS integration: resource components, system ordering, request/update/upload/event/scene-instantiation/render/audio/GC responsibilities.
- Error model: `AssetError`, `AssetIoError`, load/import/cook/bundle/GPU/dependency/cycle/type mismatch/not found/not loaded/already loaded paths.
- Feature flags: `filesystem`, `bundle`, `hot_reload`, `serde`, `async_loading`, `editor`, `importers`, individual importer features, and `parallel`.
- Examples: initialize server, load texture, load model subresources, load material with dependencies, group load, handle events, GPU upload integration, editor import, build bundle, mount bundle.

## Non-Goals

Do not make the asset module responsible for systems that consume assets:

- Render draw calls, audio playback, animation state machines, physics simulation, gameplay logic, network protocols, or the exact ECS scene instantiation policy.
- Direct renderer/audio/physics ownership of asset lifetime outside `engine_asset` handles and events.
- Declaring completion from direct path reads, typed storage alone, loader-only demos, mock-only async state, or fixed progress reports.

## Execution Rules

Follow this loop until the objective is genuinely complete:

1. Read `docs/engine_asset_api.md` and the current implementation before editing.
2. Update or create `docs/asset_goal_coverage_matrix.md` so the next work item is explicit.
3. Pick a repository-implementable `Missing`, `Stub`, or `Partial` item from the matrix.
4. Implement the smallest coherent slice that closes public API, real execution, error behavior, observability, tests, and docs together.
5. Add or update focused tests for success, failure, lifecycle, dependency, reload/unload, feature-gated behavior, and observability.
6. Run the narrow relevant tests. Broaden test scope when touching shared APIs, loader/importer infrastructure, bundle IO, GPU upload handoff, GC, ECS sync, or workspace integration.
7. Update the coverage matrix and audit report with exact evidence, remaining scope, and test commands.
8. Continue to the next uncovered item instead of declaring completion early.

Every new public API, feature flag, event, state, metadata field, loader/importer behavior, GPU upload command, example behavior, test helper, or integration hook must be added to the coverage matrix in the same slice.

## Status Rules

Use these exact meanings in the coverage matrix:

- `Implemented`: public API is reachable, real execution exists, errors are user-visible, state/events/metadata observability exists, focused tests pass, and docs are synchronized.
- `Partial`: meaningful behavior exists, but a required public API, runtime path, editor path, dependency path, GPU upload path, bundle path, error path, observability surface, test, example, or documentation link is still incomplete.
- `Stub`: API or shape exists but behavior is placeholder, mock-only, fixed-progress-only, label-only, unsupported-only, or helper-only.
- `Missing`: the documented capability is absent.
- `External Blocked`: only for true repository-external limits such as platform file watcher behavior that cannot be simulated, renderer backend GPU APIs not yet exposed, unavailable compression/codec SDKs, or OS-specific package behavior. It still requires a feature/capability gate, user-visible error, tests, matrix explanation, and follow-up entry point.

Do not mark a capability `Implemented` if the only proof is a type definition, direct file read, mock scheduler, fixed progress count, placeholder event, or documentation promise.

## Milestones

Milestone 1: Runtime MVP loop

- Create the `engine_asset` crate/module structure, core IDs, paths, `Asset` trait, handles, typed storage, errors, config, and prelude.
- Implement `AssetServer::new`, type/loader registration, `AssetIo`, filesystem IO, load/load-by-id, state tracking, events, `Assets<T>`, `get`, `state`, `is_ready`, update loop, and loader registry.
- Implement minimal built-in texture, mesh, material, shader asset types and loaders where feasible.
- Implement dependency registration for material-to-shader/texture and model subresource paths where feasible.
- Implement a minimal GPU upload queue and renderer-facing drain/finish API.
- Add focused tests and a smoke example for loading texture/mesh/material with dependencies.

Milestone 2: Editor and lifecycle baseline

- Add metadata, registry, dependency graph, load groups, progress reporting, reload, unload, GC, memory accounting, hot reload entry points, event cursors, `.meta` handling, `AssetDatabase`, importer registry, texture/model/material/audio/shader importers, cooker basics, and serde-gated references.

Milestone 3: Packaging and integration completion

- Add cooked output formats, bundle manifest/reader/writer/builder, bundle mounting, composite IO, streaming regions, memory budget eviction, DLC/patch/mod override behavior where documented, ECS integration, built-in asset type coverage, dependency visualization/audit hooks, expanded examples, audit report, and final acceptance notes.

Milestones are execution order hints, not separate completion goals. The goal remains open until the full module is complete.

## Required Tests

For each completed slice, add or update focused tests for the relevant subset:

- Public API success path produces real asset state changes.
- Missing path, missing id, missing loader, type mismatch, IO error, decode error, import error, cook error, bundle error, GPU upload error, dependency failure, cyclic dependency, already loaded, and not loaded paths.
- `AssetId`, `AssetPath`, labels/subresources, path/id mapping, registry lookup, and fallback behavior.
- Strong/weak handle behavior, untyped/typed conversion, dependency reference counts, unload protection, resident assets, and GC.
- Load scheduler priority, cancellation, deduplication, group state, group progress, and per-frame limits.
- Loader registry matching by extension and asset type, dependency/subresource registration, and loaded asset insertion.
- Importer metadata, source/settings/cooked hashes, importer version, labels, generated outputs, and dependency metadata.
- Dependency graph direct/transitive/reverse dependencies, topological order, cycle detection, and dependency failure propagation.
- Hot reload handle stability, reload events, failed reload behavior, and dependent asset behavior.
- GPU upload drain/finish, upload state transitions, upload events, failed upload, and renderer handoff.
- Bundle build/mount/read/preload, bundle IO, compression feature paths, missing bundle entries, and manifest dependencies.
- Streaming region load/unload, priority, residency, progress, and memory interaction.
- ECS component/system integration where implemented.
- Feature-gated supported and unsupported behavior.

Before final completion, run and pass:

- `cargo test -p engine_asset`
- Relevant workspace tests for crates touched by asset integration work.
- Example builds or smoke runs for asset examples.
- Renderer/audio/physics integration tests if GPU uploads or asset handles are wired into those systems.

## Acceptance Criteria

The goal can be marked complete only when all of the following are true:

- `docs/asset_goal_coverage_matrix.md` covers every capability in `docs/engine_asset_api.md` plus all public APIs, examples, tests, loader/importer APIs, bundle APIs, GPU upload APIs, events, states, metadata, and implementation-added semantics.
- No matrix item remains `Missing` or `Stub`.
- Any remaining `Partial` is a true `External Blocked` item with capability gate, user-visible error, tests, explanation, and follow-up entry point.
- Runtime code can load, query, observe, reload, unload, and GC assets through `engine_asset` public APIs.
- Editor tooling can scan, import, cook, save/load registry data, and build bundles through `engine_asset` public APIs.
- Handles remain stable across reloads, and public code does not own resource lifetimes directly.
- Paths can change without invalidating persistent asset identity when metadata/registry information exists.
- Dependencies load automatically, fail visibly, and are represented in the dependency graph.
- GPU upload is a distinct state and handoff, not hidden inside CPU decode.
- GC respects strong handles, dependency references, resident flags, and memory budgets.
- Bundles, streaming, hot reload, import/cook, ECS integration, built-in asset types, errors, tests, examples, coverage matrix, audit report, and API docs all describe the same facts.

## Stop Conditions

Do not claim completion while any of these are true:

- A repository-implementable API from `docs/engine_asset_api.md` is missing, stubbed, mock-only, unsupported-only, or unwired.
- A public feature is marked supported without a real execution path and tests.
- A public feature is marked unsupported only because it has not been implemented yet.
- Load states, events, progress, dependency graph, GPU uploads, hot reload, GC, bundle, or streaming report fake, empty, fixed, or label-only data where real data is implementable.
- Runtime asset loading and editor importing are conflated in a way that prevents separate runtime/editor use.
- CPU resource loading and GPU upload are conflated in a way that prevents renderer handoff and upload failure reporting.
- Game, renderer, audio, physics, or ECS code must bypass `engine_asset` handles to manage asset identity or lifetime.
- The coverage matrix, audit report, API doc, code, tests, or examples disagree.

## Final Response Format

When the objective is complete, report:

1. Summary of the implemented asset module.
2. Core files changed.
3. Tests and examples added or updated.
4. Exact test commands run and results.
5. Final status table for `Implemented`, `Partial`, `Stub`, `Missing`, and `External Blocked`.
6. Remaining external blockers, if any, with gates and follow-up entry points.
7. Confirmation that `docs/engine_asset_api.md`, `docs/asset_goal_coverage_matrix.md`, `docs/asset_goal_audit_report.md`, code, tests, and examples are synchronized.
