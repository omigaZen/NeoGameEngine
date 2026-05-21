# renderer_goal.md execution audit

Source of truth: `docs/rust_3d_renderer_api_design.md`.

Current goal scope: complete the renderer layer, not only the earlier six advanced graph features.

This report summarizes the current state. The detailed matrix lives in `docs/renderer_goal_coverage_matrix.md`.

## 1. Current status

The renderer layer is not complete yet.

The implementation now has broad API coverage and a verified test baseline, but several documented renderer-layer capabilities remain `Partial` because they are graph/facade-level semantics rather than complete backend-real behavior.

## 2. Verified in this session

- `cargo test -p engine_renderer`
  - Result: 197 tests passed, doc-tests passed.
- `cargo test -p engine_renderer profiler_populates_gpu_time_for_imported_extension_buffers`
  - Result: passed, including high-level GPU time populated for a facade graph with imported graph-extension buffer resources.
- `cargo test -p engine_renderer profiler_populates_gpu_time_for_imported_extension_textures`
  - Result: passed, including high-level GPU time populated for a facade graph with imported graph-extension texture resources.
- `cargo test -p engine_renderer profiler_populates_gpu_time_for_imported_environment_textures`
  - Result: passed, including high-level GPU time populated for a facade graph with imported environment texture resources.
- `cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms`
  - Result: passed, including native wgpu mesh renderer GPU timestamp stats conversion.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu -- --test-threads=1`
  - Result: 41 unit tests passed, 1 integration test passed, and doc-tests passed.
- `cargo test -p engine_renderer wgpu_metrics_`
  - Result: 2 passed, including `render_wgpu` timestamp metrics mapped into high-level `FrameStats` when profiling is enabled, hidden from high-level stats when profiling is disabled, and native wgpu pass labels preserved in `RenderGraphStats::rhi_executed_pass_labels`.
- `cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats`
  - Result: passed, including editor/debug-report exposure of backend-wgpu native pass labels, GPU profiler state/time, draw/visibility counts, reclaim policy, and `PipelineCacheStats::backend_objects`.
- `cargo test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order`
  - Result: passed, including backend-wgpu render pass label order for directional shadow cascades, spot shadows, point shadow cube faces, the final mesh pass, and no-shadow behavior when no items are visible.
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats`
  - Result: passed, including wgpu facade frame graph-stat merging so facade semantic pass labels/barriers and backend native RHI labels/timestamps/GPU time survive together.
- `cargo test -p engine_renderer initial_gpu_profiler_state_requires_timestamp_capability`
  - Result: passed, including initial `RendererConfig::gpu_profiling` state gated by timestamp capability.
- `cargo test -p engine_renderer renderer_features_cover_modern_renderer_capability_bits`
  - Result: passed, including public `RendererFeatures::SURFACE` bit coverage.
- `cargo test -p engine_renderer renderer_feature_info_reports_tiers_and_unsupported_reasons`
  - Result: passed, including public feature stability tiers, implementation levels, and unsupported reasons for core/optional/experimental/reserved features.
- `cargo test -p engine_renderer renderer_feature_infos_enumerates_all_public_features`
  - Result: passed, including runtime enumeration of every public `RendererFeature`, `RendererFeatureAudit` aggregation of total/supported/unsupported, core-supported/core-unsupported, unsupported-without-reason, backend-real/facade-semantic/graph-semantic/reserved implementation counts, supported-non-backend-real feature listing, and core/optional/experimental/reserved feature counts, plus `RendererCargoFeatureAudit` enabled/disabled aggregation and classification of all 17 `engine_renderer` Cargo features.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer renderer_feature_support_matrix -- --nocapture`
  - Result: passed, 1 test passed, including public `RendererFeatureSupportMatrix` grouping by stability tier and implementation level, explicit `supported_non_backend_real_features`, `all_supported_features_backend_real=false` for semantic graph/facade features, and explained config/reserved unsupported feature gates.
- `cargo test -p engine_renderer renderer_new_selects_configured_backend_without_surface`
  - Result: passed, including wgpu backend initialization without a surface no longer advertising `RendererFeature::Surface`.
- `cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor`
  - Result: passed, including public `FrameDebugReport` summarizing the last frame for editor/inspector use and exposing the full `RenderGraphStats` snapshot, profiler state, pipeline/material switches, upload/memory stats, submission-boundary retirement frame fields, standard frame output lists, capture trigger/request parameters, request id, queued frame index and latency, backend integration snapshot, external-hook handoff state and hook metadata, and capture label/backend/status/resource dump data from the last completed frame.
- `cargo test -p engine_renderer graph_`
  - Result: 33 tests passed, including `RenderGraphStats::semantic_passes` for graph/facade-semantic execution, `RenderGraphStats::rhi_executed_passes` for RHI/backend execution, and wgpu-backed graph/RHI tests preserving execution-kind observability.
- `cargo build -p render_facade_window_usecase`
  - Result: passed after the windowed facade example was updated to request GPU profiling, expose GPU-time/profiler gate state in the window title, and support repeatable smoke options.
- `.\target\debug\render_facade_window_usecase.exe --smoke-frames 3 --wait-for-gpu --print-stats`
  - Result: passed on the local visible-window/surface path, printing `surface-smoke frame=2 draws=1 visible=1 profiler=true gpu_time_ms=Some(0.26964) graph_passes=21 rhi_passes=21 semantic_passes=0`.
- `cargo test -p engine_renderer register_graph_extension_rejects_empty_names`
  - Result: passed, including public graph extension registration rejecting empty/blank names.
- `cargo test -p engine_renderer frame_builds_stats_from_scene_and_view`
  - Result: passed, including high-level `FrameStats::gpu_time_ms` populated from headless RHI timestamp results when GPU profiling is enabled and the graph has no imported facade resources.
- `cargo test -p engine_renderer renderer_config_controls_debug_label_groups`
  - Result: passed, confirming the RHI profiling path preserves debug-label graph stats.
- `cargo test -p engine_renderer renderer_config_controls_transient_resource_aliasing_stats`
  - Result: passed, confirming the RHI profiling path preserves transient aliasing graph stats.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer pipeline -- --nocapture`
  - Result: 18 tests passed, including shader reload, shader destroy, material template destroy, backend-wgpu native pipeline replacement tombstones, `PipelineCacheBackendCoverage`, facade/backend-object gap reporting, aggregate per-frame cache usage, and public cache entry state plus per-entry frame usage through `Renderer::pipeline_cache_entries`.
- `cargo test -p engine_renderer shader -- --nocapture`
  - Result: 18 tests passed, including public shader variant cache warmup/inspection, undeclared feature rejection, per-frame used flag reset, backend-wgpu shader module compilation for warmed variants, frame/debug/capture aggregate observability, shader reload/destroy invalidating variant entries, expanded WGSL vertex input reflection for scalar/vector f32/u32/i32 formats, explicit packed/normalized/float16 vertex formats, and the 64-bit vertex attribute unsupported/support gate path.
- `cargo test -p engine_renderer renderer_feature -- --nocapture`
  - Result: 3 tests passed, including public feature enumeration/audit counts and `RendererFeature::VertexAttribute64Bit` reporting as an optional backend-real feature with a user-visible unsupported reason when caps do not expose it.
- `cargo test -p engine_renderer wgpu_float64_vertex_attribute_pipeline_smoke_is_cap_gated -- --nocapture`
  - Result: passed; current device reported `VERTEX_ATTRIBUTE_64BIT` unavailable, so the test verified renderer caps also do not expose `RendererFeatures::VERTEX_ATTRIBUTE_64BIT` and skipped the native pipeline creation branch. On hardware exposing the feature, the same test creates a native wgpu render pipeline with a `Float64` vertex attribute layout.
- `cargo test -p engine_renderer material_creation_rejects_destroyed_template_dependencies`
  - Result: passed, including destroyed shader rejection for material template creation and destroyed template rejection for material creation.
- `cargo test -p engine_renderer material_info_reports_template_bindings_and_pipeline_readiness -- --nocapture`
  - Result: passed, including public `Renderer::material_info` / `MaterialInfo` observability for material label/domain/template handle, template readiness, standard-vs-custom material classification, parameter count, texture/sampler binding counts, pipeline readiness, and template-destroy state transitions without waiting for frame-time errors.
- `cargo test -p engine_renderer material_template_info_reports_shader_dependency_and_pipeline_readiness -- --nocapture`
  - Result: passed, including public `Renderer::material_template_info` / `MaterialTemplateInfo` observability for template label/shader/domain/render-state/pass/schema count, shader readiness, pipeline readiness, shader-destroy transitions, and destroyed-template handle disappearance.
- `cargo test -p engine_renderer material_template_schema_is_validated_against_shader_reflection -- --nocapture`
  - Result: passed, including `create_material_template` rejecting schema parameters that are absent from explicit shader reflection resources, preserving accepted texture/sampler/uniform bindings, keeping reflection-disabled handwritten schemas valid, exposing reflected binding counts plus schema/reflection coverage through `MaterialTemplateInfo`, reporting missing reflected binding counts by texture/sampler/buffer type for partially covered schemas, and exposing material-instance template/reflection coverage plus missing reflected bindings through `MaterialInfo`.
- `cargo test -p engine_renderer material_template_ -- --nocapture`
  - Result: 3 passed, covering `material_template_info_reports_shader_dependency_and_pipeline_readiness`, `material_template_schema_is_validated_against_shader_reflection`, and `frame_rejects_material_template_with_destroyed_shader`.
- `cargo test -p engine_renderer material_ -- --nocapture`
  - Result: 18 passed, covering standard/custom material validation, material/template info, schema/reflection checks, render phase selection, batching, material/pipeline switch stats, destroyed template dependencies, destroyed shader frame rejection, and full-validation material texture dependency checks.
- `cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`
  - Result: passed, including WGSL auto reflection for uniform buffers, textures, samplers, cube textures, `var<storage, read>` struct storage buffers reported as `BindingClass::Storage` / `BindingType::Buffer`, `var<push_constant>` struct ranges reported through `ShaderInterfaceDesc::push_constants`, vertex entry `@location` parameters and vertex input struct members reported through `ShaderInterfaceDesc::vertex_inputs`, and fragment `@location` parameters excluded from vertex input layout.
- `cargo test -p engine_renderer shader_file_source_is_validated_and_reflected_for_wgsl -- --nocapture`
  - Result: passed, including `.wgsl` file-source auto reflection for resource bindings, `var<push_constant>` ranges, and vertex `@location` inputs through the same `ShaderInterfaceDesc` path used by in-memory WGSL sources.
- `cargo test -p engine_renderer shader_auto_reflection_hot_reload_rejects_layout_changes -- --nocapture`
  - Result: passed, including compatible WGSL auto-reflection reload acceptance and rejection of hot reloads that change auto-reflected direct vertex inputs, vertex input struct member layouts, or push constant ranges.
- `cargo test -p engine_renderer pipeline_warmup_validates_pipeline_keys`
  - Result: passed, including DestroyQueued shader/template rejection, non-zero vertex layout hash validation, pipeline shader/material-template shader mismatch rejection, unsupported material render-phase rejection, pipeline sample-count / renderer MSAA mismatch rejection, and pipeline feature-bits subset / current-pass support validation for pipeline warmup.
- `cargo test -p engine_renderer renderer_config_rejects_invalid_latency_and_msaa`
  - Result: passed, including rejection of MSAA sample counts that cannot be represented by `PipelineKey::sample_count`.
- `cargo test -p engine_renderer capture_options_validate_backend_hooks`
  - Result: passed, including request-time external capture hook validation, explicit rejection of replacing an already queued capture request, public pending capture inspection through `Renderer::pending_frame_capture_info` with live backend status/integration/unavailable-reason data, execution-time `BackendUnavailable` status if a queued hook is removed before frame finish, public `FrameCaptureBackendInfo` availability/status queries before and after external hook registration, `FrameCapture` request-id/queued-frame/latency and backend integration snapshots, and `FrameCapture::external_hook_triggered` plus hook label/sdk handoff observability.
- `cargo test -p engine_renderer frame_rejects_material_template_with_destroyed_shader`
  - Result: passed, including frame-time rejection of a material template whose shader was destroyed after material creation.
- `cargo test -p engine_renderer render_graph_extensions`
  - Result: 3 tests passed, including destroyed import, usage mismatch, and indirect buffer import validation.
- `cargo test -p engine_renderer render_graph_extensions_reject_destroyed_imported_renderer_resources`
  - Result: passed, including destroyed texture and buffer handles imported by a custom `RenderGraphExtension`.
- `cargo test -p engine_renderer render_graph_extensions_reject_imported_renderer_resource_usage_mismatches`
  - Result: passed, including texture and buffer usage mismatches imported by a custom `RenderGraphExtension`.
- `cargo test -p engine_renderer render_graph_extensions_validate_indirect_imported_buffer_usage`
  - Result: passed, including accepted `BufferUsage::INDIRECT` and rejected non-indirect buffer imports.
- `cargo test -p engine_renderer renderer_public_flags_support_bitflag_queries`
  - Result: passed, including `BufferUsage::INDIRECT` bitflag coverage.
- `cargo test -p engine_renderer scene_command_buffer_rejects_destroyed_resource_handles_before_mutation`
  - Result: passed, including destroyed mesh/material/environment and LOD group dependency preflight.
- `cargo test -p engine_renderer scene_`
  - Result: 13 tests passed, including ECS-like extract data driving scene command buffers and producing visible frame stats.
- `cargo test -p engine_renderer ecs_like_extract_fixture_drives_scene_commands_and_frame_stats`
  - Result: passed, covering multi-entity extract, light/environment scene commands, retained scene storage, and headless frame stats from the extracted scene.
- `cargo test -p engine_renderer render_targets_must_match_renderer_msaa_samples`
  - Result: passed, covering renderer MSAA sample-count validation for direct texture targets, texture-view targets, and external render target descriptors.
- `cargo test -p engine_renderer render_targets_must_use_formats_supported_by_caps`
  - Result: passed, covering renderer format-cap validation for direct texture targets, texture-view targets, external render target descriptors at creation and frame time, headless render targets, and depth attachments.
- `cargo test -p engine_renderer texture_view_render_targets_validate_subresource_ranges`
  - Result: passed after render target sample-count validation, covering texture-view mip/layer/dimension constraints.
- `cargo test -p engine_renderer render_targets_are_validated_and_can_back_offscreen_views`
  - Result: passed after render target sample-count validation, covering offscreen render target validation and rendering.
- `cargo test -p engine_renderer generate_mips -- --nocapture`
  - Result: 4 tests passed, including retained RGBA8 2D/layered/volume mip generation, unsupported/missing data errors, and `TextureInfo::mips_generated` observability resetting after texture updates.
- `cargo test -p engine_renderer texture -- --nocapture`
  - Result: 20 tests passed, including texture creation/update validation, render-target texture validation, graph/RHI texture usage, and mip-generation observability compatibility.
- `cargo test -p engine_renderer custom_material_parameters_are_schema_validated`
  - Result: passed, including destroyed texture/sampler material update assertions.
- `cargo test -p engine_renderer standard_graph_import_rejects_destroyed_environment_textures`
  - Result: passed, including destroyed environment texture validation for graph import and frame output texture labels.
- `cargo test -p engine_renderer environment_`
  - Result: 3 tests passed, including environment IBL slot validation, environment graph import validation, profiler coverage for imported environment textures, and environment bake producing a complete retained prefiltered-specular mip chain marked through `TextureInfo::mips_generated`.
- `cargo test -p engine_renderer bindless_textures_require_capability_and_track_texture_table_pass`
  - Result: passed, including destroyed material texture validation for bindless texture table graph construction.
- `cargo test -p engine_renderer bindless`
  - Result: 2 tests passed.
- `cargo test -p engine_renderer virtual_texturing_requires_capability_and_tracks_feedback_pass`
  - Result: passed, including destroyed material texture validation for virtual texture feedback graph construction and streaming output stats.
- `cargo test -p engine_renderer virtual_texturing`
  - Result: passed.
- `cargo test -p engine_renderer resource_residency_controls_streamed_meshes_and_textures`
  - Result: passed, including direct resident/evicted resource-count observability for evict/make-resident transitions.
- `cargo test -p engine_renderer lod_frame_output_rejects_destroyed_lod_level_resources`
  - Result: passed.
- `cargo test -p engine_renderer lod`
  - Result: 3 tests passed.
- `cargo test -p engine_renderer deformation`
  - Result: 2 tests passed, including destroyed skeleton and morph resource validation for deformation stats plus frame deformation output resource-count and buffer-byte observability.
- `cargo test -p engine_renderer motion_vector`
  - Result: 3 tests passed, including destroyed mesh validation for motion vector stats and frame motion-vector output moving-mesh/vertex-byte observability.
- `cargo test -p engine_renderer frame_capture_resource_dump_counts_only_ready_resources`
  - Result: passed, including capture resource dumps mirroring `MemoryStats::resident_resources`, `MemoryStats::evicted_resources`, `MemoryStats::reclaim_policy`, `MemoryStats::delayed_destroy_bytes`, `MemoryStats::reclaimed_this_frame`, `MemoryStats::reclaimed_bytes_this_frame`, and counting ready renderer-generated mip textures through `FrameCaptureResourceDump::generated_mip_textures`.
- `cargo test -p engine_renderer frame_stats_report_resident_memory_and_delayed_destroy_count`
  - Result: passed, including `MemoryStats::reclaim_policy`, `MemoryStats::delayed_destroy_bytes`, `MemoryStats::reclaimed_this_frame`, and `MemoryStats::reclaimed_bytes_this_frame` reporting frame-latency reclamation, delayed-memory pressure, and zero reclaim while resources remain delayed.
- `cargo test -p engine_renderer generic_resource_lifecycle_covers_public_resource_kinds`
  - Result: passed, including `ResourceReclaimPolicy::FrameLatency`, `MemoryStats::delayed_destroy_bytes` remaining stable during frame-latency delay, then moving into `MemoryStats::reclaimed_bytes_this_frame` with the exact reclaimed count when resources become invalid.
- `cargo test -p engine_renderer wait_for_gpu_reclaims_destroy_queued_resources_without_frame_latency`
  - Result: passed, including `FrameInput::wait_for_gpu` flushing pending uploads, reporting `ResourceReclaimPolicy::BackendFence`, and reclaiming DestroyQueued resources before configured frame latency after backend/headless GPU synchronization.
- `cargo test -p engine_renderer submitted_frame_ -- --nocapture`
  - Result: 2 tests passed, covering both default submitted-frame upload/staging completion and empty default frames preserving DestroyQueued resources while submitted frames reclaim them through `ResourceReclaimPolicy::SubmissionBoundary` when the submission boundary is complete.
- `cargo test -p engine_renderer poll_resource_retirements_completes_only_prior_submission_work -- --nocapture`
  - Result: passed, covering explicit non-blocking retirement polling, per-submission upload batch accounting, and future-frame DestroyQueued resources staying delayed until covered by a later completed submission boundary.
- `cargo test -p engine_renderer wgpu_metrics_`
  - Result: 2 tests passed, including wgpu backend frame stats carrying an explicit `ResourceReclaimPolicy` and native `rhi_executed_pass_labels`.
- `cargo test -p engine_renderer frame_wait_for_gpu_flushes_pending_upload_stats`
  - Result: passed, including `UploadStats::bytes_queued_this_frame`, `UploadStats::uploads_queued_this_frame`, `UploadStats::staging_bytes_queued_this_frame`, `UploadStats::bytes_uploaded_this_frame`, `UploadStats::uploads_completed_this_frame`, and `UploadStats::staging_bytes_released_this_frame` resetting on the next frame so frame-local upload data does not leak across frames.
- `cargo test -p engine_renderer upload_stats_track_pending_staging_until_flush`
  - Result: passed, including manual `flush_uploads` preserving immediate queued-this-frame, uploaded-this-frame, completed-upload-count, staging-queued, and released-staging-byte observability outside frame boundaries.
- `cargo test -p engine_renderer capture`
  - Result: 2 tests passed.
- `cargo test -p engine_renderer capture -- --nocapture`
  - Result: 2 tests passed, including external capture hook gating, explicit pending-capture replacement rejection, public pending capture inspection through `Renderer::pending_frame_capture_info` with live backend status/integration/unavailable-reason data, execution-time unavailable status, public SDK/dependency metadata through `FrameCaptureIntegration`, `FrameCaptureBackendInfo::sdk_name`, and `FrameCaptureBackendInfo::unavailable_reason`, request-id/queued-frame/latency and frame-finish backend integration snapshots through `FrameCapture`, registered external-hook metadata through `FrameCaptureHookDesc`, `Renderer::register_frame_capture_backend_hook`, `Renderer::unregister_frame_capture_backend_hook`, `registered_hook_label`, and `registered_sdk_name`, `FrameCapture::external_hook_triggered` plus hook label/sdk handoff observability, plus `FrameCapture::pipeline_cache` preserving per-frame pipeline cache stats in capture payloads.
- `cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`
  - Result: passed, including pass-level `RenderGraphStats::rhi_executed_pass_labels` evidence for standard 3D passes entering the profiled Headless RHI path.
- `cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`
  - Result: passed, including editor-facing exposure of pass-level RHI labels and per-frame `PipelineCacheStats`.
- `cargo test -p engine_renderer graph_ -- --nocapture`
  - Result: 33 tests passed, including graph/RHI compatibility after adding `RenderGraphStats::rhi_executed_pass_labels`.
- `cargo build -p render_scene_usecase -p render_facade_usecase -p render_facade_window_usecase -p render_smoke -p render_feature_showcase`
  - Result: passed.

## 3. New or tightened implementation evidence

- `Renderer::with_surface` is now covered by `examples/render_facade_window_usecase`.
- `render_facade_window_usecase` now has repeatable smoke verification options: `--smoke-frames`, `--wait-for-gpu`, `--print-stats`, and `--require-gpu-time`.
- `render_facade_usecase` covers a user-facing `RenderGraphExtension` custom pass.
- GPU profiler enable now requires `RendererFeature::TimestampQuery`.
- Initial `RendererConfig::gpu_profiling` also requires timestamp capability before the wgpu runtime reports profiler enabled; unsupported wgpu devices are explicitly degraded to disabled profiler state.
- `RendererFeatures::SURFACE` is now represented in the caps bitset, and wgpu renderers created without `Renderer::with_surface` no longer report `RendererFeature::Surface`.
- `RendererFeatureTier`, `RendererFeatureImplementation`, `RendererFeatureInfo`, and `Renderer::feature_info` now expose per-feature support, stability tier, implementation level, and unsupported reason instead of leaving feature stability as docs-only state.
- `RendererFeature::all`, `Renderer::feature_infos`, and `RendererFeatureAudit` expose the complete runtime feature/stability list plus aggregate supported/unsupported, core-supported/core-unsupported, unsupported-without-reason, backend-real/facade-semantic/graph-semantic/reserved implementation counts, supported-non-backend-real feature listing, and tier counts for audits and tools.
- `RendererFeatureTierSupport`, `RendererFeatureImplementationSupport`, `RendererFeatureSupportMatrix`, and `Renderer::feature_support_matrix()` expose a product-facing support matrix grouped by stability tier and implementation level, keeping backend-real, facade-semantic, graph-semantic, config-gated, and reserved capabilities explicitly separated.
- `renderer_cargo_feature_infos` and `RendererCargoFeatureAudit` expose enabled/disabled state and classify every `engine_renderer` Cargo feature as a runtime feature gate, config runtime gate, reserved backend, base facade feature, or reserved tooling feature.
- Missing GPU timestamp data keeps `FrameStats::gpu_time_ms` as `None`; no synthetic `0.0ms` fallback is reported.
- Disabled GPU profiling suppresses backend timestamp values from high-level `FrameStats`, avoiding stale or residual GPU-time reports after profiler state changes.
- Facade graph profiling now uses a headless RHI timestamp path, mapping imported facade texture/buffer resources into headless RHI imports when needed, including standard environment textures and graph-extension texture/buffer imports, preserving transient aliasing and debug-label stats while populating high-level `FrameStats::gpu_time_ms`.
- Native wgpu mesh rendering now writes encoder start/end timestamps when profiling is enabled, resolves them to readback buffers, exposes `MeshRenderStats::gpu_time_ns`, and maps that value into high-level `FrameStats::gpu_time_ms` / `graph.gpu_time_ns`.
- `render_facade_window_usecase` now explicitly requests GPU profiling and shows draw/visible/GPU-time or profiler-gate state in the window title, giving the visible surface path a user-observable profiling signal.
- Vulkan, Metal, and D3D12 backend preferences now return explicit `UnsupportedFeature` until real backends exist.
- External frame capture backends now require an available registered hook before a capture request is queued.
- A second frame capture request is now rejected while another capture is pending, so capture requests cannot be silently overwritten before frame finish.
- `Renderer::pending_frame_capture_info` exposes the pending capture request id, label, backend, options, queued frame index, and current backend status/integration/unavailable-reason data before frame finish.
- Queued external frame captures recheck hook availability at frame finish and report `BackendUnavailable` if the hook was removed after queuing.
- `FrameCaptureHookDesc`, `Renderer::register_frame_capture_backend_hook`, and `Renderer::unregister_frame_capture_backend_hook` now expose explicit external-hook registration metadata while preserving the compatibility `set_frame_capture_backend_available` path.
- `FrameCaptureBackend::all`, `FrameCaptureBackendInfo`, `FrameCaptureIntegration`, `Renderer::frame_capture_backend_info`, and `Renderer::frame_capture_backend_infos` now expose capture backend availability, external-hook requirements, SDK/dependency names, registered hook labels, registered SDK names, unavailable reasons, and request-time status for tools before a capture is queued.
- `FrameCapture::external_hook_triggered`, `FrameCapture::external_hook_label`, `FrameCapture::external_hook_sdk_name`, and the matching `FrameDebugReport` fields now expose whether a registered external capture hook handoff was still available and requested at frame finish, plus which hook/sdk was handed off.
- `FrameCapture` now snapshots request id, queued frame index, capture latency, backend integration, external-hook requirement, SDK name, and unavailable reason at frame finish, so capture payloads remain self-describing even if hook registration changes later.
- Public renderer resources use frame-latency delayed destroy at the facade/headless layer before old handles become invalid.
- `ResourceReclaimPolicy`, `MemoryStats::delayed_destroy_bytes`, `MemoryStats::reclaimed_this_frame`, and `MemoryStats::reclaimed_bytes_this_frame` now expose the current frame-latency reclamation strategy, queued delayed-destroy byte pressure, plus the number and bytes of delayed resources actually reclaimed on the current frame; `FrameCaptureResourceDump` mirrors those values for capture/tooling output.
- `FrameCaptureResourceDump::generated_mip_textures` counts Ready textures whose current mip chain was generated by `Renderer::generate_mips`, so capture payloads can observe generated mip resources instead of only total texture counts.
- `FrameInput::wait_for_gpu` now waits/polls the backend when present, preferring the latest recorded wgpu `SubmissionIndex` when available, flushes pending upload staging, reports `ResourceReclaimPolicy::BackendFence` through frame memory stats, and performs zero-latency reclaim of DestroyQueued resources after explicit backend synchronization.
- Default submitted frames now distinguish completed submission-boundary resource release from explicit GPU idle: empty frames preserve DestroyQueued resources, completed submitted work reclaims them with `ResourceReclaimPolicy::SubmissionBoundary`, and backend-wgpu uses a non-blocking submission poll before taking that path.
- `Renderer::poll_resource_retirements` exposes explicit non-blocking completed-boundary retirement outside frame finish, returning upload/memory snapshots plus retired/pending submission frame information. Submitted uploads are now batched by submission frame, so uploads queued after a boundary and resources destroyed in a future frame remain pending until a later completed boundary covers them.
- FrameStats, FrameDebugReport, and FrameCapture mirror etired_submission_frame and pending_submission_frame, so tools and capture artifacts can observe completed-boundary retirement progress without calling the poll API directly.
- Main surface lifecycle is covered as a renderer-owned resource: status/priority queries, stale surface rejection, priority updates, explicit destroy rejection for the main surface, invalid stale-surface destroy, invalid `RenderTarget::Surface`, and resized main-surface extent propagation.
- Render target validation now enforces renderer MSAA sample-count compatibility and renderer format-cap compatibility for direct texture targets, texture-view targets, external render target descriptors, headless targets, and depth attachments, preventing frame-time pipeline sample-count and unsupported format mismatches.
- Texture mip generation now has public observability through `TextureInfo::mips_generated` and capture observability through `FrameCaptureResourceDump::generated_mip_textures`; successful retained mip generation sets it and later texture updates clear it so tools can distinguish generated mip chains from freshly updated base/subresource data.
- Environment bake now writes a complete retained prefiltered-specular mip chain for the requested mip count and marks that texture as renderer-generated mips, rather than declaring multiple mip levels while storing only the base-level bytes.
- `FrameDebugReport` now provides a public editor-facing frame debugger summary from `last_frame_stats`, including the full `RenderGraphStats` snapshot, graph labels/counts/barriers, pass-level `rhi_executed_pass_labels`, draw/dispatch/visibility counts, profiler state/GPU time, pipeline/material switches, upload/memory stats, submission-boundary retirement frame fields, standard frame output lists, debug draw outputs, picking outputs, capture trigger/request parameters, capture label/backend/status/request-id/queued-frame/latency/backend integration/external-hook handoff metadata/resource dump data, and pipeline statistics.
- Backend-wgpu frame stats are now covered through `FrameDebugReport`, preserving native pass labels, profiler state/time, draw/visibility counts, reclaim policy, and backend pipeline object counts for editor tooling.
- `FrameStats::pipeline_cache` and `FrameDebugReport::pipeline_cache` expose per-frame pipeline cache hit/miss/invalidation/backend-object stats for frame tooling.
- `FrameCapture::pipeline_cache` carries the same per-frame pipeline cache stats into capture payloads.
- `FrameStats`, `FrameDebugReport`, and `FrameCapture` now expose shader variant cache entry count, variants used this frame, backend-compiled variant count, and unique shader interface layout count, so editor tools and capture artifacts can diagnose variant cache pressure without enumerating all variants.
- `ShaderVariantInfo::backend_compiled` reports whether a warmed shader variant has a backend-native compiled module. Backend-wgpu now owns a per-shader/per-feature `wgpu::ShaderModule` variant cache, reuses duplicate warmups, and invalidates cached modules on shader reload/destroy.
- Public `VertexFormat` and WGSL auto reflection now cover common scalar/vector f32, u32, and i32 vertex input formats and route those formats through shader interface hashing, facade backend-wgpu vertex layout mapping, and RHI wgpu pipeline mapping.
- Public `VertexFormat` explicit interfaces also cover wgpu packed/normalized/float16 storage formats (`Uint8`/`Sint8`/`Unorm8`/`Snorm8`, signed/normalized 16-bit vectors, and `Float16x2/4`) and route them through shader interface hashing, facade backend-wgpu vertex layout mapping, and RHI wgpu pipeline mapping.
- `RendererFeature::VertexAttribute64Bit` / `RendererFeatures::VERTEX_ATTRIBUTE_64BIT` now gate `Float64*` vertex formats. Backend-wgpu requests `wgpu::Features::VERTEX_ATTRIBUTE_64BIT` when available, reports it through caps only when enabled, and shader creation/reload returns `UnsupportedFeature(VertexAttribute64Bit)` when a shader interface uses 64-bit vertex attributes without support.
- `wgpu_float64_vertex_attribute_pipeline_smoke_is_cap_gated` provides backend-native conditional coverage for `Float64` vertex attributes: unsupported devices verify caps-gated skip behavior, and supporting devices create the native pipeline.
- `PipelineCacheStats::backend_objects` exposes native backend render pipeline object counts; backend-wgpu fills it from `render_wgpu::MeshRenderer::render_pipeline_count`.
- Wgpu facade frames now merge renderer facade cache totals/hits/misses/invalidations with backend-wgpu native pipeline inventory, so frame stats do not lose either facade cache behavior or native backend object observability.
- `PipelineCacheStats::entries_used_this_frame` and `PipelineCacheStats::ready_unused_entries` expose aggregate frame usage for tools that do not enumerate individual cache entries.
- `PipelineCacheStats::ready_entries_without_backend_object` and `PipelineCacheStats::used_entries_without_backend_object` expose the current facade/backend-object gap at both ready-entry and frame-used-entry granularity.
- `Renderer::pipeline_cache_entries` exposes `PipelineCacheEntryInfo` / `PipelineCacheEntryStatus` so tools can inspect individual cached pipeline keys and see that current facade-created entries have no backend pipeline object yet.
- `PipelineCacheEntryInfo::last_used_frame` and `PipelineCacheEntryInfo::used_this_frame` expose per-entry frame usage, distinguishing warmup-only cache entries from pipeline keys actually consumed by the current frame.
- `RenderGraphStats::semantic_passes` and `RenderGraphStats::rhi_executed_passes` now distinguish graph/facade-semantic passes from RHI/backend-executed passes, and `FrameDebugReport` exposes those counts for editor tooling.
- `RenderGraphStats::rhi_executed_pass_labels` now preserves pass-level RHI execution evidence; `frame_builds_stats_from_scene_and_view` verifies standard 3D passes including `gpu_culling`, `gpu_deformation`, `depth_prepass`, `gbuffer`, `ssao`, `deferred_lighting`, `motion_vectors`, `taa`, and `present` entered the profiled Headless RHI path.
- Backend-wgpu frame stats now preserve native wgpu `RenderPassDescriptor` labels in `RenderGraphStats::rhi_executed_pass_labels`, including directional shadow cascades, spot shadows, point shadow cube faces, and the final mesh pass; skybox work remains observable inside `Neo Mesh Pass` rather than as a fake separate pass.
- Wgpu facade frames now merge facade semantic graph stats with backend-wgpu native RHI graph stats, so `FrameStats::graph` retains facade pass labels/barriers plus backend executed pass labels/timestamps/GPU time.
- `SceneCommandBuffer` now prevalidates command resource handles before mutating retained scene state, so destroyed mesh/material/environment handles are rejected at command application time instead of being written into the scene and discovered later during frame rendering.
- `ExtractRenderData` now has an integration-style ECS-like fixture that emits multi-object, light, and environment scene commands and verifies the extracted retained scene through headless frame stats.
- LOD group dependency validation is reused for frame scene preflight and scene command assignment, so destroyed level mesh/material resources are rejected before a stale LOD group can be assigned to an object.
- `FrameLodOutput` construction also validates LOD group dependencies, so validation-off frames cannot report a destroyed selected LOD mesh.
- Deformation stats now validate skeleton, morph, and selected mesh resources before reporting skinned/morphed/deformed frame outputs.
- `FrameDeformationOutput` now reports unique skeleton/morph resource counts and skeleton/morph buffer-byte footprints, so GPU deformation frame output is observable beyond object counts and the deformed vertex output buffer.
- Motion-vector stats now validate selected mesh resources before reporting moving object frame outputs.
- `FrameMotionVectorOutput` now reports moving mesh counts and vertex-byte footprints, so motion-vector frame output exposes the mesh resources that feed the pass while camera-only/TAA/motion-blur outputs remain explicit zero-resource cases.
- Material parameter create/update validation is now explicitly tested against destroyed texture and sampler handles, not only fabricated missing handles.
- Material template creation now rejects DestroyQueued shader handles, and material creation now rejects DestroyQueued material template handles.
- Pipeline warmup now requires shader and material template handles to be Ready, so DestroyQueued shader/template handles are rejected before entering the pipeline cache.
- Pipeline warmup now rejects zero `PipelineKey::vertex_layout_hash`, matching renderer-generated keys that derive this value from mesh handles.
- Pipeline warmup now verifies that `PipelineKey::shader` matches the material template shader, so mismatched shader/template keys cannot enter the pipeline cache.
- Pipeline warmup now verifies that `PipelineKey::pass` is supported by the material template pass flags, so explicit warmup cannot cache a phase the material would never generate.
- Pipeline warmup now verifies that `PipelineKey::sample_count` matches renderer MSAA, and renderer config rejects MSAA values that cannot be represented by `PipelineKey::sample_count`.
- Pipeline warmup now verifies that `PipelineKey::feature_bits` is a non-empty subset of material template pass flags and supports `PipelineKey::pass`, so explicit warmup stays compatible with material-generated pipeline keys, including standard-material pass subsets.
- Destroying a shader or material template now invalidates dependent warmed pipeline cache entries, matching shader hot reload invalidation behavior.
- Frame draw-item/pipeline-key generation now revalidates material template shader handles, so destroying a shader after material creation cannot leave stale pipeline keys in frame stats or cache bookkeeping.
- Standard view graph environment texture imports now validate renderer texture readiness before importing external texture handles, so destroyed environment textures are rejected during graph construction.
- Environment frame output texture labels now validate renderer texture readiness before reporting labels for skybox/IBL resources.
- Environment frame outputs now carry skybox/irradiance/prefiltered-specular/BRDF mip counts and generated-mip state, so retained IBL bake output is observable through frame stats, debug reports, and captures.
- `MemoryStats::resident_resources` and `MemoryStats::evicted_resources` now expose resource residency transitions directly, and frame capture resource dumps mirror those counts for tools.
- `MemoryStats` and `FrameCaptureResourceDump` now expose streamable resource totals, resident/evicted streamable resource counts, resident/evicted streamable texture mip counts, and resident/evicted streamable mesh byte totals, so streaming residency is observable outside successful render-view output construction.
- `cargo test -p engine_renderer resource_residency_controls_streamed_meshes_and_textures -- --nocapture`: passed, including resident/evicted streamable resource counts, texture mip counts, and mesh byte totals.
- `cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed, including `FrameCaptureResourceDump` mirroring the new streamable residency fields from `MemoryStats`.
- Custom `RenderGraphExtension` imports now validate renderer texture/buffer readiness and usage before graph execution, so extension code cannot import destroyed facade resources or resources with undeclared usage into custom passes.
- Public graph extension registration now has explicit test coverage for rejecting empty extension names.
- Public `BufferUsage` now exposes `INDIRECT`, and custom graph extension indirect buffer imports require that facade usage bit before graph execution.
- Bindless texture table graph construction now validates material texture readiness before enumerating imported texture handles.
- Virtual texture feedback graph construction now validates material texture readiness before using streamable texture metadata.
- Frame streaming output now validates streamable mesh and material texture readiness before reporting streaming stats.
- Frame capture resource dumps count only Ready resources while reporting DestroyQueued resources through `delayed_destroy_count`.
- Wgpu-backed graph/RHI tests are serialized with a test-only guard to avoid Windows teardown instability from concurrent device creation/destruction.

## 4. Implemented areas by current evidence

- Type-safe handles.
- Mesh and buffer retained APIs.
- Scene retained mode and scene command buffer.
- Scene command-buffer stale/destroyed resource preflight for mesh/material/environment paths.
- LOD group stale/destroyed internal mesh/material dependency preflight.
- LOD frame output stale/destroyed selected mesh validation.
- Deformation stats stale/destroyed skeleton, morph, and selected mesh validation.
- Deformation frame output skeleton/morph resource-count and buffer-byte observability.
- Motion-vector stats stale/destroyed selected mesh validation.
- Motion-vector frame output moving-mesh count and vertex-byte observability.
- Material update stale/destroyed texture and sampler validation.
- Material template/material creation stale/destroyed dependency validation.
- Pipeline warmup stale/destroyed shader and material template validation.
- Pipeline warmup non-zero vertex layout validation.
- Pipeline warmup shader/material-template coherence validation.
- Pipeline warmup material pass flag/render-phase coherence validation.
- Pipeline warmup sample-count / renderer MSAA coherence validation.
- Pipeline warmup feature-bits subset / current-pass support validation.
- Pipeline cache invalidation on shader and material template destruction.
- Pipeline cache aggregate frame usage observability through `entries_used_this_frame` and `ready_unused_entries`.
- Pipeline cache aggregate backend-object gap observability through `ready_entries_without_backend_object` and `used_entries_without_backend_object`.
- Pipeline cache per-entry frame usage observability through `last_used_frame` and `used_this_frame`.
- Frame-time material template shader dependency validation.
- Environment texture graph-import stale/destroyed handle validation.
- Environment frame output texture label stale/destroyed handle validation.
- Environment frame output mip-count and generated-mip-state observability for skybox/IBL resources.
- Custom graph extension imported texture/buffer stale/destroyed handle and usage mismatch validation.
- Graph extension registration empty-name validation.
- Public indirect buffer usage and custom graph extension indirect buffer import validation.
- Bindless texture table stale/destroyed material texture validation.
- Virtual texture feedback stale/destroyed material texture validation.
- Streaming output stale/destroyed mesh and material texture validation.
- Resident/evicted resource-count observability through `MemoryStats` and frame capture resource dumps.
- Capture resource dump ready-only resource counting with delayed-destroy reporting.
- Explicit reclamation policy, delayed-destroy byte pressure, and current-frame resource reclamation count/byte reporting through `MemoryStats` and capture resource dumps.
- High-level frame profiling from headless RHI timestamp results for facade graphs, including imported environment textures, graph-extension textures, and graph-extension buffers.
- Native wgpu mesh renderer timestamp metric conversion and backend-to-`FrameStats` mapping.
- Camera, view, and render target descriptors.
- Renderer-owned main surface lifecycle semantics.
- Editor-facing frame debug reports from the last completed frame.
- Editor-facing full graph statistics visibility through `FrameDebugReport`.
- Editor-facing semantic-vs-RHI graph pass visibility and pass-level RHI execution labels through `FrameDebugReport`.
- Pass-level RHI execution labels through `RenderGraphStats::rhi_executed_pass_labels`.
- Editor-facing profiler state and capture request parameter visibility through `FrameDebugReport`.
- Editor-facing external capture hook handoff and hook metadata visibility through `FrameDebugReport`.
- Capture request-id, queued-frame, latency, and backend integration snapshot visibility through `FrameCapture` and `FrameDebugReport`.
- Pending capture replacement rejection.
- Pending capture request public inspection through `Renderer::pending_frame_capture_info`, including live backend status/integration gate data.
- Editor-facing pipeline/material switch, upload, and memory visibility through `FrameDebugReport`.
- Editor-facing culling, SSAO, light-cluster, area-light, ray-tracing, shadow, gbuffer, LOD, streaming, environment, deformation, motion-vector, and post-process output visibility through `FrameDebugReport`.
- Editor-facing capture metadata and resource dump visibility through `FrameDebugReport`.
- Repeatable visible-window/surface profiler verification through `render_facade_window_usecase --smoke-frames 3 --wait-for-gpu --print-stats`.
- Public feature stability/gate information through `Renderer::feature_info`.
- Public aggregate feature/stability audit information through `Renderer::feature_audit`, including core feature closure, unsupported-reason completeness, implementation-level counts, and supported-non-backend-real feature listing.
- Public Cargo feature classification through `Renderer::cargo_feature_audit`.
- Public capture backend availability/status information through `Renderer::frame_capture_backend_info`.
- RenderGraph builder/pass/context and graph extension APIs.
- Custom pass example coverage.
- Error handling through `Result`/`RendererError`.
- Example set builds.

## 5. Partial areas

- Renderer facade config/init/caps:
  - Wgpu and headless paths exist.
  - Vulkan/Metal/D3D12 are explicit unsupported features, not real backends.
- Texture/sampler:
  - Retained texture APIs and CPU mip generation exist.
  - Backend-real mip generation and streaming are not complete.
- Shader/material/pipeline:
  - Reflection, hot reload compatibility, material templates, shader variant cache APIs, and cache stats exist.
  - Backend-wgpu reflected pipeline invalidation is now wired for shader reload/destroy and material-template destroy through native cache batch invalidation; shader variant cache is implemented at renderer layer and future work is deeper variant-to-native-pipeline specialization.
- Light/shadow/environment:
  - Directional/point/spot/area/environment descriptors and graph outputs exist.
  - IBL/environment bake output is now visible through texture info and frame environment outputs, but is not complete for every facade/backend path.
- Animation/skinning/morph/LOD:
  - Data and graph/frame outputs exist.
  - Frame deformation and motion-vector outputs now report input resource footprints, but backend deformation/skinning/motion-vector shader-buffer execution still needs deeper closure.
- Standard 3D graph:
  - Deferred/Forward+ graph structure, pass ordering, post-process labels, stats, and tests exist.
  - Semantic graph passes are now observable separately from RHI/backend-executed passes.
  - Some passes are still renderer-layer graph semantics rather than full backend shader implementations.
- RHI/backend abstraction:
  - Headless and wgpu RHI paths exist with tests.
  - RHI graph execution and native wgpu metrics now report backend-executed pass counts separately from semantic graph pass counts.
  - Higher-level facade frame graph is not fully executing standard 3D passes through backend RHI.
- GPU memory/upload/streaming:
  - Upload stats, flush, priority, residency, and delayed destroy exist.
  - Resident/evicted resource counts are now directly observable through `MemoryStats` and capture dumps.
  - Frame-latency delayed resource strategy, byte pressure, and reclamation are now observable through `ResourceReclaimPolicy`, `MemoryStats::delayed_destroy_bytes`, `MemoryStats::reclaimed_this_frame`, `MemoryStats::reclaimed_bytes_this_frame`, and capture resource dumps.
  - Explicit `FrameInput::wait_for_gpu` now closes the GPU-idle reclaim path by waiting/polling the backend when present, flushing pending staging, reporting `ResourceReclaimPolicy::BackendFence`, and reclaiming delayed resources before configured frame latency.
  - Default submitted frames now reclaim DestroyQueued facade resources at completed submission boundaries with `ResourceReclaimPolicy::SubmissionBoundary`; backend-wgpu gates this through non-blocking submission polling, and empty frames remain on frame-latency delayed destroy.
  - `Renderer::poll_resource_retirements` exposes explicit non-blocking completed-boundary polling for upload and DestroyQueued retirement outside frame finish, with per-submission upload batching and future-frame DestroyQueued protection.
  - FrameStats, FrameDebugReport, and FrameCapture mirror retired/pending submission-boundary frame indices for tooling and capture observability.
  - `UploadStats::bytes_queued_this_frame`, `UploadStats::uploads_queued_this_frame`, `UploadStats::staging_bytes_queued_this_frame`, `UploadStats::bytes_uploaded_this_frame`, `UploadStats::uploads_completed_this_frame`, and `UploadStats::staging_bytes_released_this_frame` now reset at frame start so upload data is frame-local while manual `flush_uploads` remains observable outside frame boundaries.
  - Backend-wgpu reflected native pipeline invalidation, replacement, shader variant module invalidation, and material external texture/sampler unregister now move native objects into backend-owned tombstones; renderer-level `MemoryStats`, capture dumps, and `Renderer::poll_resource_retirements` expose and drive backend tombstone retirement after completed submission polling. Cooperative background retirement startup/observability is supported, while true backend fence objects/nonblocking per-submission completion queries and any remaining backend-owned resource classes are still incomplete.
- Profiling/capture:
  - Stats/capture structures and gates exist.
  - Headless RHI timestamp plumbing now reaches high-level frame stats for facade graphs, including imported environment textures, graph-extension textures, and graph-extension buffers.
  - Native wgpu mesh renderer timestamp plumbing now reaches high-level frame stats through backend metrics mapping and has repeatable visible-window/surface verification through `render_facade_window_usecase --smoke-frames 3 --wait-for-gpu --print-stats`.
  - Capture backend availability/status, integration kind, SDK/dependency name, registered hook metadata, unavailable reason, request-id/queued-frame/latency/backend integration snapshot, and external-hook handoff metadata are now queryable through public API, but only registered external hook handoff is implemented.
  - Real RenderDoc SDK and external debugger SDK invocation remain incomplete and explicitly `Partial`.

## 6. Stub or missing areas

- Real RenderDoc or external debugger integration.
- Real Vulkan backend.
- Real Metal backend.
- Real D3D12 backend.
- Full backend-real implementation for ray tracing, mesh shader, virtual texturing, variable rate shading, and bindless beyond current renderer-layer graph/capability/stat semantics.
- Full editor frame debugger.

## 7. Current highest-priority next work

1. Replace registered capture-hook metadata/handoff with real RenderDoc/external debugger SDK invocation where available; until then, keep this area explicitly `Partial` with public SDK/dependency metadata, registered hook metadata, and unavailable reasons.
2. Decide whether advanced backend features should remain explicit `UnsupportedFeature` until real backend support exists, or implement real backend paths.
3. Move upload/delayed-destroy from submitted-frame bookkeeping/explicit completed-boundary polling and reflected native pipeline tombstones toward true backend fence objects/nonblocking per-submission completion queries and tombstones for any remaining backend-owned resource classes.
4. Use semantic-vs-RHI graph stats and `rhi_executed_pass_labels` to continue converting graph/RHI-command standard pass semantics into backend shader/material implementations where practical.
5. Keep the repeatable window smoke command in final example verification on machines with a visible desktop.

## 8. Completion decision

Do not mark `renderer_goal.md` complete yet.

The current codebase has a strong verified baseline, but the complete renderer-layer objective still has documented `Partial`, `Stub`, and `Missing` items.
- `cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`
  - Result: passed, including WGSL auto reflection of `texture_storage_2d` as `BindingClass::Storage` / `BindingType::Texture`, alongside sampled textures, storage buffers, push constants, and vertex inputs.
- `cargo test -p engine_renderer material_template_schema_is_validated_against_shader_reflection -- --nocapture`
  - Result: passed, including explicit shader reflection and material parameter validation for storage texture bindings using `TextureHandle`.
- `cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`
  - Result: passed, including WGSL auto reflection of `texture_1d` and `texture_storage_1d` as `TextureDimension::D1`.
- `cargo test -p engine_renderer material_template_schema_is_validated_against_shader_reflection -- --nocapture`
  - Result: passed, including material/schema reflection coverage for D1 sampled and storage texture bindings.
- `cargo test -p engine_renderer material_template_ -- --nocapture`
  - Result: passed, including `MaterialTemplateInfo::shader_interface_layout_hash` coverage for shader resources, push constants, and vertex inputs exposed to material-template/pipeline diagnostics.
- `cargo test -p engine_renderer material_info_reports_template_bindings_and_pipeline_readiness -- --nocapture`
  - Result: passed, including `MaterialInfo::shader_interface_layout_hash` coverage for ready materials and zero hash after template destruction.
- `cargo test -p engine_renderer pipeline -- --nocapture`
  - Result: passed, including `PipelineCacheEntryInfo::shader_interface_layout_hash` coverage for pipeline cache entry diagnostics.
- `cargo test -p engine_renderer pipeline -- --nocapture`
  - Result: passed, including `PipelineCacheStats::shader_interface_layouts` aggregate coverage aligned with per-entry shader interface layout hashes.
- `cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`
  - Result: passed, including `FrameDebugReport::pipeline_shader_interface_layouts` mirroring `PipelineCacheStats::shader_interface_layouts` for editor/inspector pipeline layout observability.
- `cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`
  - Result: passed, including `FrameCapture::pipeline_shader_interface_layouts` mirroring `PipelineCacheStats::shader_interface_layouts` for capture artifact pipeline layout observability.

## 本轮验证：render_wgpu 材质 layout contract
- 变更：`Render/render_wgpu/src/mesh_renderer.rs`、`Render/render_wgpu/src/lib.rs` 增加材质 bind group layout 可观测 API 与 shader binding contract 测试。
- 命令：`cargo test -p render_wgpu material_backend_layout_info_matches_mesh_shader_bindings -- --nocapture`
- 结果：passed，1 passed；`mesh.wgsl` 中 `@binding(0..=30)` 与公开的 material layout contract 匹配。
- 未覆盖：没有完成高层 `engine_renderer` material-template pipeline layout 到 `render_wgpu` 动态 bind group 的后端创建与提交路径。

## 本轮验证补强：实际 wgpu material layout 使用同一 contract
- 变更：`MeshRenderer::new` 的 material bind group layout 创建改为复用 `material_bind_group_layout_entries()`。
- 命令：`cargo test -p render_wgpu material_backend_layout_info_matches_mesh_shader_bindings -- --nocapture`
- 结果：passed，1 passed；测试覆盖公开 layout info、实际 layout entries helper、`mesh.wgsl` binding 声明三者一致。

## 本轮验证补强：material resource entries 复用 contract
- 变更：`WgpuMaterial::new` 中实际 bind group resource entries 改为使用 `MATERIAL_UNIFORM_BINDING`、`MATERIAL_TEXTURE_BINDINGS`、`MATERIAL_SAMPLER_BINDINGS`。
- 命令：`cargo test -p render_wgpu material_backend_layout_info_matches_mesh_shader_bindings -- --nocapture`
- 结果：passed，1 passed；编译覆盖实际 resource entry 的 contract 常量引用，测试继续验证 layout info、layout entries 和 shader bindings 一致。

## 本轮验证：wgpu backend pipeline layout inventory stats
- 变更：`Render/render_wgpu/src/mesh_renderer.rs` 增加静态 pipeline layout inventory；`Render/engine_renderer/src/backend_wgpu.rs` 将 layout count 写入 `PipelineCacheStats.shader_interface_layouts`。
- 命令：`cargo test -p render_wgpu mesh_renderer_reports_static_pipeline_inventory -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer wgpu_metrics_map_gpu_timestamps_to_frame_stats -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`
- 结果：passed，1 passed。
- 未覆盖：没有完成动态 `engine_renderer` material-template/shader reflection pipeline 到 `render_wgpu` native bind group/pipeline creation 的接线。

## 本轮验证：shader group/binding 与 wgpu layout plan
- 变更：`Render/engine_renderer/src/lib.rs` 为 `ShaderResourceBinding` 增加 `group`/`binding`，auto reflection 解析 WGSL slot，validation 增加重复 slot 检查，layout hash 纳入 slot。
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 增加 `WgpuShaderInterfaceLayoutPlan`、`WgpuShaderBindGroupLayoutPlan`、`wgpu_shader_interface_layout_plan()`。
- 命令：`cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer shader_reflection_accepts_explicit_interface_and_validates_entry_points -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer material_template_schema_is_validated_against_shader_reflection -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer shader_file_source_is_validated_and_reflected_for_wgsl -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer wgpu_shader_interface_layout_plan_maps_reflected_groups_and_bindings -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer wgpu_shader_interface_layout_plan_rejects_unmapped_backend_bindings -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer shader_hot_reload_updates_compatible_shader_and_invalidates_pipeline_cache -- --nocapture`
- 结果：passed，1 passed。
- 未覆盖：storage texture format/access reflection 与实际 native bind group/pipeline layout object creation 尚未完成。

## 本轮验证：storage texture format/access reflection 与 wgpu layout mapping
- 变更：`Render/engine_renderer/src/lib.rs` 增加 `BindingType::StorageTexture`、`StorageTextureAccess`，WGSL auto reflection 解析 storage texture format/access，material texture dimension validation 支持 storage texture，layout hash 纳入 format/access。
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 将 `BindingType::StorageTexture` 映射到 `wgpu::BindingType::StorageTexture`，并保留 unsupported storage format rejection。
- 命令：`cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer wgpu_shader_interface_layout_plan_maps_reflected_groups_and_bindings -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer material_template_schema_is_validated_against_shader_reflection -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer wgpu_shader_interface_layout_plan_rejects_unmapped_backend_bindings -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer shader_hot_reload_updates_compatible_shader_and_invalidates_pipeline_cache -- --nocapture`
- 结果：passed，1 passed。
- 未覆盖：actual wgpu bind group layout object creation、pipeline layout object creation、render pipeline object creation、material parameters to native bind group resource binding。

## 本轮验证：material bind group resource planning
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 增加 material parameter 到 backend bind group resource entry 的 plan 类型与转换函数。
- 命令：`cargo test -p engine_renderer wgpu_material_bind_group_resource_plan_maps_parameters_to_reflected_slots -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer wgpu_material_bind_group_resource_plan_rejects_unbound_or_mismatched_parameters -- --nocapture`
- 结果：passed，1 passed。
- 未覆盖：actual `wgpu::BindGroup` object creation、GPU texture/sampler/buffer resource lookup、native pipeline submission path。

## 本轮验证：native layout/bind group creation API 编译覆盖
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 增加 reflected shader layout object creation 与 material bind group creation API。
- 命令：`cargo test -p engine_renderer wgpu_shader_interface_layout_plan_maps_reflected_groups_and_bindings -- --nocapture`
- 结果：passed，1 passed；编译覆盖 native layout object creation API。
- 命令：`cargo test -p engine_renderer wgpu_material_bind_group_resource_plan_maps_parameters_to_reflected_slots -- --nocapture`
- 结果：passed，1 passed；编译覆盖 native bind group creation API。
- 未覆盖：runtime resource table lookup、actual GPU object creation smoke test、render pipeline object creation、pipeline cache integration、final render submission path。

## 本轮验证：render pipeline creation API 编译覆盖
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 增加 native `wgpu::RenderPipeline` creation API。
- 命令：`cargo test -p engine_renderer wgpu_shader_interface_layout_plan_maps_reflected_groups_and_bindings -- --nocapture`
- 结果：passed，1 passed；编译覆盖 render pipeline creation API。
- 未覆盖：actual GPU render pipeline object creation smoke test、shader module creation、pipeline cache integration、final render submission path。

## 本轮验证：shader module creation API 编译覆盖
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 增加 native `wgpu::ShaderModule` creation API。
- 命令：`cargo test -p engine_renderer wgpu_shader_interface_layout_plan_rejects_unmapped_backend_bindings -- --nocapture`
- 结果：passed，1 passed；编译覆盖 shader module creation API。
- 未覆盖：actual GPU shader module creation smoke test、non-WGSL translation、pipeline cache integration、final render submission path。

## 本轮验证：wgpu native pipeline cache metadata/stats
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 增加 native pipeline cache metadata 与 stats 汇总。
- 命令：`cargo test -p engine_renderer wgpu_native_pipeline_cache_metadata_reports_backend_stats -- --nocapture`
- 结果：passed，1 passed。
- 未覆盖：actual wgpu handle cache ownership、runtime pipeline cache integration、render submission path。

## 本轮验证：WgpuRendererRuntime native pipeline cache stats integration
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 让 `WgpuRendererRuntime` 持有 native pipeline cache metadata，并将其 stats 合并进 `FrameStats.pipeline_cache`。
- 命令：`cargo test -p engine_renderer wgpu_native_pipeline_cache_metadata_reports_backend_stats -- --nocapture`
- 结果：passed，1 passed。
- 命令：`cargo test -p engine_renderer wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory -- --nocapture`
- 结果：passed，1 passed。
- 未覆盖：actual wgpu handle cache ownership、runtime pipeline creation invocation、resource lookup、final render submission path。

## 本轮验证：runtime reflected pipeline build-and-cache API 编译覆盖
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 增加 actual wgpu pipeline handle ownership 与 `create_and_cache_native_render_pipeline()`。
- 命令：`cargo test -p engine_renderer wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory -- --nocapture`
- 结果：passed，1 passed；编译覆盖 handle ownership 字段/API。
- 命令：`cargo test -p engine_renderer wgpu_native_pipeline_cache_metadata_reports_backend_stats -- --nocapture`
- 结果：passed，1 passed；编译覆盖 build-and-cache API。
- 未覆盖：actual GPU object creation smoke test、runtime resource lookup、material bind group auto creation、final render submission path。

## 本轮验证：material bind group auto creation 编译覆盖
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 保留 material buffer payload、自动创建 owned native buffers、并在 runtime build-and-cache 入口中支持 material resource plan。
- 命令：`cargo test -p engine_renderer wgpu_material_bind_group_resource_plan_maps_parameters_to_reflected_slots -- --nocapture`
- 结果：passed，1 passed；验证 resource plan 保存 bytes payload 并按 reflected slots 排序。
- 命令：`cargo test -p engine_renderer wgpu_native_pipeline_cache_metadata_reports_backend_stats -- --nocapture`
- 结果：passed，1 passed；编译覆盖 runtime build-and-cache material bind group auto creation API。
- 未覆盖：actual GPU bind group smoke test、texture/sampler runtime resource table lookup、final render submission path。

## 本轮验证：runtime material resource lookup 与 submission binding API
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 增加 material external resource registry、registered-resources build path、cached pipeline submission lookup、render-pass binding helper。
- 命令：`cargo test -p engine_renderer wgpu_material_bind_group_resource_plan_rejects_unbound_or_mismatched_parameters -- --nocapture`
- 结果：passed，1 passed；编译覆盖 registered-resources build path。
- 命令：`cargo test -p engine_renderer wgpu_material_external_resource_registry_reports_missing_handles -- --nocapture`
- 结果：passed，1 passed；验证 missing texture/sampler handles 返回 `InvalidHandle`。
- 命令：`cargo test -p engine_renderer wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory -- --nocapture`
- 结果：passed，1 passed；编译覆盖 submission lookup/bind helper API。
- 未覆盖：actual GPU bind group/pipeline/render-pass smoke test、automatic `render_scene()` reflected pipeline submission path。

## 本轮验证：actual wgpu reflected pipeline smoke
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 新增 actual GPU reflected pipeline smoke test。
- 命令：`cargo test -p engine_renderer wgpu_reflected_pipeline_smoke_creates_and_binds_native_objects -- --nocapture`
- 结果：passed，1 passed；本机实际创建并提交 reflected wgpu render pass，未走 skip 分支。
- 未覆盖：automatic `render_scene()` reflected pipeline submission path / scene queue integration。

## 本轮验证：runtime draw-to-view reflected pipeline submission
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 增加 `WgpuNativePipelineDrawDesc` 与 `WgpuRendererRuntime::submit_native_pipeline_draw_to_view()`，actual smoke test 改为通过 runtime API 提交。
- 命令：`cargo test -p engine_renderer wgpu_reflected_pipeline_smoke_creates_and_binds_native_objects -- --nocapture`
- 结果：passed，1 passed；真实创建 reflected wgpu pipeline 并通过 runtime draw-to-view API 提交 render pass。
- 未覆盖：automatic `render_scene()` scene queue integration。

## 本轮验证：render_scene queued reflected draw integration 编译覆盖
- 变更：`Render/render_wgpu/src/mesh_renderer.rs` 增加 post-pass hook；`Render/render_wgpu/src/scene.rs` 增加 `render_with_post_pass()`；`Render/engine_renderer/src/backend_wgpu.rs` 增加 queued native pipeline draw，并在 `render_scene()` 中 drain 到 MeshRenderer post-pass。
- 命令：`cargo test -p engine_renderer wgpu_reflected_pipeline_smoke_creates_and_binds_native_objects -- --nocapture`
- 结果：passed，1 passed；继续验证 actual reflected GPU pipeline smoke 与 runtime direct submission path。
- 命令：`cargo test -p engine_renderer wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory -- --nocapture`
- 结果：passed，1 passed；编译覆盖 render_scene queued submission 接线与 stats merge。
- 未覆盖：with-surface/window-backed `render_scene()` reflected queue smoke test；高层 scene/material 到 queued reflected draw 的自动调度策略。

## 本轮审计增量：facade reflected material 自动调度

新增证据：
- `Render/engine_renderer/src/lib.rs` 现在能从 retained scene 的 custom material draw item 生成 wgpu reflected native draw plan，并在 main-surface facade 渲染前通过 `WgpuRendererRuntime` 创建/缓存 native reflected pipeline、owned buffer-backed bind group、queued draw。
- `cargo test -p engine_renderer reflected_facade -- --nocapture` 当时通过，覆盖自动计划生成和 mesh vertex-input shader 的显式错误路径；mesh vertex/index buffer binding 已在后续 reflected texture/sampler material 增量中闭合。
- `cargo test -p engine_renderer wgpu_reflected_pipeline_smoke_creates_and_binds_native_objects -- --nocapture` 通过，继续覆盖 actual wgpu reflected pipeline 创建和提交。

后续闭合：
- facade 自动 reflected draw 路径的 mesh vertex/index buffers、renderer texture/sampler native registration、with-surface/window-backed `render_scene()` reflected queue smoke 已在后续增量中补齐；该段保留为历史阶段记录。

完成判断：
- `renderer_goal.md` 仍不能标记完成；本轮只是关闭了高层 scene/material 到 backend queued draw 的第一段可实现子路径。

## 本轮审计增量：窗口 usecase 覆盖

新增证据：
- `Examples/render_facade_window_usecase/src/main.rs` 已接入 procedural WGSL reflected custom material，由 public renderer facade 创建 shader/material template/material 并放入 retained scene。
- `cargo build -p render_facade_window_usecase` 通过，说明示例编译覆盖新的 facade reflected material 创建路径。

后续闭合：
- GUI/surface smoke、renderer-managed texture/sampler native registration 与 mesh vertex/index buffer binding 已在后续增量中补齐；该段保留为历史阶段记录。

## 本轮审计增量：surface smoke 结果

新增证据：
- 修复了 facade reflected pipeline 使用推断 color format 而不是真实 swapchain format 的问题；现在 native reflected pipeline 创建优先使用 `WgpuRendererRuntime::surface_color_format()`。
- `target\debug\render_facade_window_usecase.exe --smoke-frames 3 --wait-for-gpu --print-stats` 通过，exit 0，并输出 `draws=3`、`visible=2`、`pipeline_cache_total=2`、`shader_layouts=2`。

完成判断：
- with-surface/window-backed reflected queue smoke 已有一次本机验证通过。
- `renderer_goal.md` 仍不能标记完成；本段提到的 reflected custom material mesh vertex/index buffer binding 与 renderer texture/sampler native registration 已在后续条目闭合，但能力矩阵中仍有其他 Partial/Stub/Missing 项。

## 本轮审计增量：reflected texture/sampler material

新增证据：
- backend-wgpu 现在可以从 renderer texture/sampler 描述创建 native `wgpu::TextureView` / `wgpu::Sampler` 并注册给 reflected material bind group 构建。
- facade reflected draw path 现在接受 TextureHandle/SamplerHandle 参数，并在 native pipeline 创建前自动注册这些资源。
- generated mip-chain reflected texture upload 现在优先为 2D RGBA8/BGRA8 material texture 生成 base-level upload + backend GPU mip generation descriptor，并由 `wgpu_reflected_facade_texture_upload_uses_gpu_mip_generation_for_2d_rgba8` 覆盖 facade descriptor 语义。
- backend-wgpu reflected material texture upload now supports a real GPU mip-generation path for D2/D2Array/Cube/CubeArray RGBA8/BGRA8 sampled material textures: the facade upload descriptor sends only the base mip, sets `generate_mips_from_base`, and `create_wgpu_material_texture_binding` renders the remaining mip levels/layers on the GPU while exposing the generated count through `WgpuMaterialTextureBinding::generated_mips`.
- `cargo test -p engine_renderer wgpu_reflected_facade_texture_upload_uses_gpu_mip_generation_for_2d_rgba8 -- --nocapture`: passed.
- `cargo test -p engine_renderer wgpu_material_texture_binding_generates_mips_on_gpu -- --nocapture`: passed.
- `cargo test -p engine_renderer wgpu_material_array_texture_binding_generates_layer_mips_on_gpu -- --nocapture`: passed.
- `cargo test -p engine_renderer wgpu_material_cube_texture_binding_generates_face_mips_on_gpu -- --nocapture`: passed.
- `cargo test -p engine_renderer wgpu_material_texture_gpu_mip_generation_rejects_invalid_descs -- --nocapture`: passed.
- reflected custom material 现在会校验 shader vertex input 与 mesh vertex layout，创建 native vertex layout，上传并绑定 renderer mesh vertex/index bytes；`wgpu_reflected_facade_draws_bind_mesh_vertex_index_buffers` 覆盖 draw plan 中的 vertex/index buffer payload。
- renderer-owned texture 现在维护 revision；`update_texture`、`generate_mips` 和环境 bake mip 标记会推进 revision，facade reflected native key 纳入 TextureHandle 对应 revision，避免 texture 内容更新后复用绑定旧 `TextureView` 的 native bind group。
- backend-wgpu 现在将 structural native render pipeline object cache 与 material bind group entry cache 分开；`wgpu_native_cache_reuses_render_pipeline_across_material_bind_groups` 验证两个 material bind group entries 复用一个 `wgpu::RenderPipeline`，`PipelineCacheStats::backend_objects` 报告 unique native render pipeline object 数量。
- backend-wgpu native reflected pipeline cache 现在支持按 ShaderHandle / MaterialTemplateHandle / MaterialHandle 批量失效；renderer shader reload/destroy、material template destroy 和 material parameter update 会同步清理匹配的 active native pipeline entries，并将失效对象移入 backend-owned tombstones；不再被 active entry 引用的 unique `wgpu::RenderPipeline` 会从 active cache 移除，但实际 backend 对象由 tombstone 持有到 poll retirement。
- `cargo test -p engine_renderer reflected_facade -- --nocapture` 通过，4 个相关测试覆盖 bytes-only、texture/sampler、texture update 后 native key 刷新、mesh vertex/index buffer binding 和 generated mip-chain upload splitting。
- `cargo test -p engine_renderer wgpu_native_cache_reuses -- --nocapture` 通过，1 个 backend-wgpu cache 复用测试覆盖 render pipeline object / bind group object 生命周期拆分、shader/template/material 批量失效后的 active cache 清理、backend-owned tombstone 计数，以及显式 poll 退休 tombstoned shader module/layout/bind group/owned buffer/render-pipeline refs。
- cargo test -p engine_renderer wgpu_shader_variant_module_cache -- --nocapture 通过，1 个 backend-wgpu shader variant module cache 测试覆盖重复 warmup 复用 native wgpu::ShaderModule、shader invalidation 后 variant modules 移入 backend-owned tombstone，以及显式 poll 退休 tombstoned variant modules。
- `cargo test -p engine_renderer material -- --nocapture` 通过，24 个 material 相关测试继续覆盖 material update/schema/resource validation。
- `cargo test -p engine_renderer pipeline -- --nocapture` 通过，14 个 pipeline 相关测试继续覆盖 shader reload/destroy/template destroy 的 renderer facade cache 失效路径。
- `cargo build -p render_facade_window_usecase` 通过。
- `render_facade_window_usecase --smoke-frames 3 --wait-for-gpu --print-stats` 通过，exit 0；窗口 smoke 已实际执行采样 renderer texture/sampler 且绑定 mesh vertex/index buffers 的 reflected custom material。

仍未完成：
- Wgpu surface/direct submissions now record the latest `wgpu::SubmissionIndex`, and `WgpuRendererRuntime::wait_for_gpu()` uses `WaitForSubmissionIndex` for that submission before falling back to a device-wide wait; `wgpu_reflected_pipeline_smoke_creates_and_binds_native_objects` asserts a direct submission index is recorded before waiting.
- 更长期的 per-upload/per-resource backend resource residency、剩余资源类别 tombstone、真实 fence-backed automatic background lifetime 管理仍在 GPU memory/upload 项下跟踪。
- `renderer_goal.md` 仍不能标记完成；能力矩阵仍包含其他 Partial/Stub/Missing 项。





## 本轮进展：backend-wgpu material external resource tombstones
- 变更：`WgpuRendererRuntime::unregister_material_texture_binding` 和 `unregister_material_sampler_binding` 会把 native texture view / sampler binding 移入 backend-owned tombstone，并通过 `poll_backend_resource_retirements` 显式退休。
- `cargo test -p engine_renderer wgpu_material_external_resources_unregister_into_backend_tombstones -- --nocapture`：passed，覆盖 texture/sampler unregister 后 tombstone 计数与显式 poll retirement。
- 回归验证：`cargo test -p engine_renderer wgpu_shader_variant_module_cache -- --nocapture` passed；`cargo test -p engine_renderer wgpu_native_cache_reuses -- --nocapture` passed。


## 本轮进展：renderer-level backend tombstone observability
- 变更：新增 renderer-level `BackendResourceRetirementStats`，从 backend-wgpu `WgpuBackendResourceRetirementStats` 映射到 `MemoryStats.backend_retirement` 和 `FrameCaptureResourceDump.backend_retirement`。
- 变更：`Renderer::poll_resource_retirements()` 同时驱动 backend-wgpu tombstone retirement，使高层 API 可以观测并推进 backend tombstone 生命周期。
- `cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture`：passed，覆盖 renderer-level memory stats、capture dump 和 high-level poll 对 backend tombstone retirement 的观测。
- 回归验证：`cargo test -p engine_renderer wgpu_material_external_resources_unregister_into_backend_tombstones -- --nocapture` passed；`cargo test -p engine_renderer wgpu_shader_variant_module_cache -- --nocapture` passed；`cargo test -p engine_renderer wgpu_native_cache_reuses -- --nocapture` passed。

## 本轮进展：backend tombstone fence object observability
- 变更：backend-owned tombstone 记录 backend fence object，携带最新 `wgpu::SubmissionIndex`（如果 runtime 已有提交），并通过 `BackendResourceRetirementStats.fence_objects` / `retired_fence_objects_this_poll` 暴露 live 和 retired fence object 数量。
- `cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture`：passed。
- `cargo test -p engine_renderer wgpu_material_external_resources_unregister_into_backend_tombstones -- --nocapture`：passed。
- `cargo test -p engine_renderer wgpu_shader_variant_module_cache -- --nocapture`：passed。
- `cargo test -p engine_renderer wgpu_native_cache_reuses -- --nocapture`：passed。
- 剩余：finer per-fence non-blocking completion、剩余 backend-owned resource classes tombstone 仍在 GPU memory/upload 项下跟踪；cooperative background retirement startup/observability 已由后续条目关闭。

## 本轮进展：frame-begin backend tombstone maintenance
- 变更：`Renderer::begin_frame()` 自动执行非阻塞 backend tombstone maintenance，使空帧也可以推进已完成 backend tombstone/fence retirement，并通过 `FrameStats.memory.backend_retirement` 发布 retired counts。
- `cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture`：passed。
- 回归验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed。
- 剩余：finer per-fence non-blocking completion、剩余 backend-owned resource classes tombstone 仍在 GPU memory/upload 项下跟踪；cooperative background retirement startup/observability 已由后续条目关闭。

## 本轮进展：backend-wgpu post-pass buffer tombstones
- 变更：reflected native pipeline post-pass submission 的临时 vertex/index `wgpu::Buffer` 现在在 `render_scene()` 提交完成后进入 backend-owned tombstone，并由 `poll_backend_resource_retirements()` 退休。
- 变更：backend-wgpu 和 renderer-level backend retirement stats 现在暴露 `post_pass_vertex_buffers`、`post_pass_index_buffers`、`retired_post_pass_vertex_buffers_this_poll`、`retired_post_pass_index_buffers_this_poll`。
- 验证：`cargo test -p engine_renderer wgpu_post_pass_buffers_enter_backend_tombstones_until_poll_retirement -- --nocapture` passed，1 passed。
- 剩余：GPU memory/upload/streaming 仍是 Partial；后续需要继续收敛 finer per-fence non-blocking completion，以及尚未进入 backend-owned tombstone 的剩余 backend resource 类别；cooperative background retirement startup/observability 已由后续条目关闭。
- 回归验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，确认 renderer-level `MemoryStats` / capture dump 映射仍可观测 backend tombstone retirement。
- 回归验证：`cargo test -p engine_renderer wgpu_native_cache_reuses -- --nocapture` passed，确认 reflected native pipeline cache tombstone 路径未回退。
- 新增映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，确认 backend-wgpu post-pass buffer retirement stats 映射到 renderer-level `BackendResourceRetirementStats` 不丢字段。

## 本轮进展：backend-wgpu material external resource replacement tombstones
- 变更：material external texture/sampler binding 重新注册同一 handle 时，旧 native binding 进入 backend-owned tombstone；覆盖直接注册和 create-and-register 两条路径。
- 验证：`cargo test -p engine_renderer wgpu_material_external_resources_replace_into_backend_tombstones -- --nocapture` passed，1 passed。
- 回归验证：`cargo test -p engine_renderer wgpu_material_external_resources_unregister_into_backend_tombstones -- --nocapture` passed，1 passed。
- 剩余：该项继续推进了 backend-owned resource class tombstone 覆盖，但 renderer goal 仍未完成；finer per-fence non-blocking completion、剩余 backend-owned resource tombstone coverage 和更广的 renderer 层 Partial/Stub/Missing 仍在队列中。
- 补充验证：`cargo test -p engine_renderer wgpu_material_sampler_create_and_register_replaces_into_backend_tombstone -- --nocapture` passed，覆盖 create-and-register sampler replacement 的旧 native sampler tombstone 入队和 poll retirement。
- 补充验证：`cargo test -p engine_renderer wgpu_material_texture_create_and_register_replaces_into_backend_tombstone -- --nocapture` passed，覆盖 create-and-register texture replacement 的旧 native texture binding tombstone 入队和 poll retirement。

## 本轮进展：backend tombstone fence-index observability
- 变更：`WgpuBackendResourceRetirementStats` 和 renderer-level `BackendResourceRetirementStats` 增加 indexed/unindexed fence 计数，区分带 `wgpu::SubmissionIndex` 的 tombstone fence 与无 submission index 的 tombstone fence。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，1 passed。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，1 passed。
- 剩余：该项只闭合 per-fence observability，不等同于完整 finer per-fence non-blocking completion；当前 backend-wgpu 仍以 queue-empty non-blocking poll 作为安全 retirement 条件。
- 高层观测验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，确认 indexed/unindexed fence 统计通过 renderer-level `MemoryStats.backend_retirement` 和 `FrameCaptureResourceDump.backend_retirement` 可观测。
- 自动维护高层验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，确认 `Renderer::begin_frame()` 自动 backend tombstone maintenance 产生的 `FrameStats.memory.backend_retirement` 同样暴露 indexed/unindexed fence 细分 retired counts。
- Debug report 观测验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，现断言 `FrameDebugReport.memory.backend_retirement` 与 `FrameStats.memory.backend_retirement` 一致暴露自动 maintenance 的 unindexed fence retired count。
- FrameCapture 观测验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，现断言 queued internal `FrameCapture.resource_dump.backend_retirement`、`FrameStats.memory.backend_retirement` 和 `FrameDebugReport.memory.backend_retirement` 一致暴露自动 maintenance 的 unindexed fence retired count。

## 本轮进展：backend tombstone queue-empty poll gate observability
- 变更：`BackendResourceRetirementStats` 新增 `last_poll_queue_empty` 和 `retired_after_queue_empty_poll`，使 backend tombstone retirement 的 queue-empty gate 可通过 backend stats、renderer memory stats、frame capture resource dump 和 debug report 观察。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，1 passed。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，1 passed。
- 高层观测验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，1 passed。
- 剩余：queue-empty gate observability 不是完整 per-fence completion；renderer goal 仍未完成。

## 本轮进展：backend tombstone queue-empty gate invalidation
- 变更：backend tombstone 入队会失效旧的 queue-empty poll gate 统计，避免新 pending tombstone 错误继承旧 poll 结果。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_enqueue_invalidates_previous_queue_empty_gate -- --nocapture` passed，1 passed。
- 回归验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，1 passed。
- 回归验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，1 passed。
- 剩余：该项修正当前 gate observability 语义，不等同于完整 per-fence completion；renderer goal 仍未完成。

## 本轮进展：backend tombstone completed-submission-index gate observability
- 变更：`BackendResourceRetirementStats` 新增 `last_poll_completed_submission_index_recorded` 和 `retired_after_completed_submission_index_poll`，区分 queue-empty retirement 是否绑定到已记录 backend submission index。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，1 passed。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，1 passed。
- 高层观测验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，1 passed。
- 自动维护验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，1 passed。
- 剩余：这是 completed-submission-index gate observability，不是完整 per-fence completion；renderer goal 仍未完成。
- 补充验证：`cargo test -p engine_renderer wgpu_backend_tombstone_enqueue_invalidates_previous_completed_submission_gate -- --nocapture` passed，覆盖旧 completed-submission-index poll gate 不能错误覆盖新入队 tombstone。

## 本轮进展：completed-submission-index gate false-positive fix
- 修复：`retired_after_completed_submission_index_poll` 不再因为 poll 前出现后续 unrelated submission index 而误标 unindexed tombstone；它只对本次 retired set 中带 indexed fence 的 tombstone 置 true。
- 验证：`cargo test -p engine_renderer wgpu_backend_unindexed_tombstone_does_not_claim_completed_submission_retirement -- --nocapture` passed，1 passed。
- 回归验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，1 passed。
- 剩余：仍不是完整 per-fence completion；renderer goal 仍未完成。

## 本轮进展：tombstone-level indexed/unindexed fence coverage stats
- 变更：`BackendResourceRetirementStats` 新增 live/retired tombstone-level indexed/unindexed 计数，使工具无需从 fence 数量间接推断 tombstone set 覆盖情况。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，1 passed。
- false-positive 回归：`cargo test -p engine_renderer wgpu_backend_unindexed_tombstone_does_not_claim_completed_submission_retirement -- --nocapture` passed，1 passed。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，1 passed。
- 剩余：这仍是 observability，不是完整 per-fence non-blocking completion；renderer goal 仍未完成。

## 本轮进展：all-tombstones submission-index coverage flags
- 变更：`BackendResourceRetirementStats` 新增 `all_tombstones_have_submission_index` 和 `retired_all_tombstones_had_submission_index_this_poll`，直接表达 pending/retired tombstone set 是否全部具备 submission-index fence。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，1 passed。
- false-positive 回归：`cargo test -p engine_renderer wgpu_backend_unindexed_tombstone_does_not_claim_completed_submission_retirement -- --nocapture` passed，1 passed。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，1 passed。
- 剩余：这仍是 observability，不是完整 per-fence non-blocking completion；renderer goal 仍未完成。
- 高层观测验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，现覆盖 `FrameStats`、`FrameCapture.resource_dump` 和 `FrameDebugReport` 对 `all_tombstones_have_submission_index` / `retired_all_tombstones_had_submission_index_this_poll` 的观测。
- 显式 poll 高层观测验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，覆盖 `Renderer::memory_stats()` 与 `Renderer::poll_resource_retirements()` 对 tombstone-level indexed/unindexed coverage 字段和 all-indexed retired flag 的观测。
- mixed set 语义验证：`cargo test -p engine_renderer wgpu_backend_mixed_tombstone_set_reports_partial_submission_index_coverage -- --nocapture` passed，覆盖同一 pending/retired tombstone set 同时包含 indexed 与 unindexed tombstone 时，partial coverage 计数、`retired_after_completed_submission_index_poll=true` 和 `retired_all_tombstones_had_submission_index_this_poll=false` 的组合语义。

## 本轮进展：partial submission-index coverage flags
- 变更：`BackendResourceRetirementStats` 新增 `partial_tombstone_submission_index_coverage` 和 `retired_partial_tombstone_submission_index_coverage_this_poll`，直接表达 mixed indexed/unindexed tombstone coverage。
- 验证：`cargo test -p engine_renderer wgpu_backend_mixed_tombstone_set_reports_partial_submission_index_coverage -- --nocapture` passed，1 passed。
- 回归验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，1 passed。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，1 passed。
- 剩余：这仍是 observability，不是完整 per-fence non-blocking completion；renderer goal 仍未完成。

## 本轮进展：no-indexed tombstone coverage flags
- 变更：`BackendResourceRetirementStats` 新增 `no_tombstones_have_submission_index` 和 `retired_no_tombstones_had_submission_index_this_poll`，与 all/partial flags 共同直接表达 tombstone submission-index coverage 三态。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，1 passed。
- mixed 回归：`cargo test -p engine_renderer wgpu_backend_mixed_tombstone_set_reports_partial_submission_index_coverage -- --nocapture` passed，1 passed。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，1 passed。
- 剩余：这仍是 observability，不是完整 per-fence non-blocking completion；renderer goal 仍未完成。
- 高层观测验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，覆盖 `Renderer::memory_stats()` 与 `Renderer::poll_resource_retirements()` 对 `no_tombstones_have_submission_index` / `retired_no_tombstones_had_submission_index_this_poll` 的观测。
- 自动维护高层验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，覆盖 `FrameStats`、`FrameCapture.resource_dump` 与 `FrameDebugReport` 对 no-indexed coverage 字段的镜像。

## 本轮进展：tombstone submission-index coverage enum
- 变更：`BackendResourceRetirementStats` 新增 live/retired `TombstoneSubmissionIndexCoverage` enum 字段，直接表达 `NotApplicable` / `None` / `Partial` / `All` 四态。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，1 passed。
- mixed 验证：`cargo test -p engine_renderer wgpu_backend_mixed_tombstone_set_reports_partial_submission_index_coverage -- --nocapture` passed，1 passed。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，1 passed。
- 剩余：这仍是 observability，不是完整 per-fence non-blocking completion；renderer goal 仍未完成。
- 高层观测验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，覆盖 `Renderer::memory_stats()` 与 `Renderer::poll_resource_retirements()` 对 `TombstoneSubmissionIndexCoverage::None` 的观测。
- 自动维护高层验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，覆盖 `FrameStats`、`FrameCapture.resource_dump` 和 `FrameDebugReport` 对 coverage enum 的镜像。
- NotApplicable 语义验证：`cargo test -p engine_renderer wgpu_backend_tombstone_enqueue_invalidates_previous_queue_empty_gate -- --nocapture` passed，覆盖 zero tombstone set 的 `TombstoneSubmissionIndexCoverage::NotApplicable`，以及新入队 unindexed tombstone 后 coverage 转为 `None`。
- 高层 NotApplicable 验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，覆盖 tombstone 全部退休后 `Renderer::memory_stats().backend_retirement.tombstone_submission_index_coverage == NotApplicable`。
- idle poll reset 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_enqueue_invalidates_previous_queue_empty_gate -- --nocapture` passed，现覆盖 tombstone 退休后的下一次 backend idle poll 将 retired coverage enum 重置为 `NotApplicable`。
- 高层 idle poll reset 验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，覆盖 `Renderer::poll_resource_retirements()` 在无 backend tombstone retirement 时不会保留上一轮 retired coverage 状态。
- enum helper 直接验证：`cargo test -p engine_renderer wgpu_tombstone_submission_index_coverage_enum_covers_all_states -- --nocapture` passed，覆盖 `NotApplicable`、`None`、`Partial`、`All` 四态映射，防止 coverage enum 与 raw counts / predicate fields 漂移。

## 2026-05-19 本轮进展：后台 resource retirement 能力边界显式化

- `RendererFeature::BackgroundResourceRetirement` 和 `RendererFeatures::BACKGROUND_RESOURCE_RETIREMENT` 已加入 public capability 体系。
- 历史状态：此条目最初将 `Renderer::start_background_resource_retirement()` 记录为 unsupported-only 边界。
- 当前状态：后续 2026-05-20 条目已实现 cooperative background retirement startup/observability，包括 lightweight scheduler thread、start/stop API、safe-point tick consumption、feature/caps support 和 memory/retirement stats active-state observability。
- 剩余边界：true nonblocking backend completion queries 仍不可用，当前 wgpu backend 仍以 queue-empty fallback 作为稳定 retirement gate。

## 2026-05-19 本轮进展：backend-wgpu tombstone per-fence retirement 过滤

- backend-wgpu resource tombstone retirement 不再只依赖 queue-empty 后整体 drain；每个带 wgpu `SubmissionIndex` 的 tombstone 会捕获 renderer 内部单调 submission order。
- retirement 现在通过 tombstone 自己的 fence order 与 completed submission order 判断是否可释放；未达到 completed order 的 tombstone 会保留在 pending 队列。
- wgpu 0.20.1 的 `SubmissionIndex` 不可排序，因此内部 order 只作为 renderer retirement 边界，不替代 public submission-index 观测字段。
- unindexed tombstone 仍只在 queue-empty 时释放，避免把后续无关 submission 的完成误报为该 tombstone 的 completed-submission retirement。
- 新增验证：`wgpu_backend_retirement_filters_tombstones_by_completed_submission_index`，并回归 `wgpu_backend_tombstone_enqueue_invalidates_previous_queue_empty_gate`、`wgpu_backend_unindexed_tombstone_does_not_claim_completed_submission_retirement`、`wgpu_backend_mixed_tombstone_set_reports_partial_submission_index_coverage`。
- 状态：GPU memory / upload / delayed destroy 项继续保持 `Partial`；per-fence filtering 已闭合，跨线程 worker 和更广泛 renderer 层完整性仍未完成。

## 2026-05-19 本轮进展：backend tombstone pending 原因 public observability

- `WgpuBackendResourceRetirementStats` 和 public `BackendResourceRetirementStats` 新增 `tombstones_waiting_for_submission_index` 与 `tombstones_waiting_for_queue_empty`。
- 这两个字段把 live tombstone 的等待原因暴露到 `MemoryStats.backend_retirement`、frame capture resource dump 和 debug/report 传播路径：带 fence 的资源等待自身 submission order，无 submission index 的资源等待 queue-empty。
- per-fence retirement 过滤现在不只在 backend 内部测试可见，也能通过 renderer facade 的 stats/capture 观测 pending 原因。
- 新增/回归验证：`wgpu_backend_retirement_filters_tombstones_by_completed_submission_index`、`renderer_backend_retirement_stats_map_post_pass_buffers`、`renderer_memory_stats_expose_backend_tombstone_retirement`。
- 状态：GPU memory / upload / delayed destroy 仍为 `Partial`；pending 原因观测已闭合，跨线程 worker、更多资源类覆盖和完整 renderer 层闭环仍未完成。

## 2026-05-19 本轮进展：backend retirement poll 粒度显式公开

- `WgpuBackendResourceRetirementStats` 与 public `BackendResourceRetirementStats` 新增：`nonblocking_submission_index_poll_supported`、`queue_empty_poll_fallback`、`last_poll_used_queue_empty_fallback`。
- 当前 wgpu 0.20.1 backend 明确报告 `nonblocking_submission_index_poll_supported = false`、`queue_empty_poll_fallback = true`；public `poll_resource_retirements()` 只能稳定使用 queue-empty 粒度确认完成。
- 内部 per-fence order 过滤仍保留，用于在可提供 completed order 的路径上只释放自身 fence 已完成的 tombstone；公开 stats 不再误导为已具备真正非阻塞 per-submission 查询。
- 新增/回归验证：`wgpu_backend_retirement_filters_tombstones_by_completed_submission_index`、`renderer_backend_retirement_stats_map_post_pass_buffers`、`renderer_memory_stats_expose_backend_tombstone_retirement`。
- 状态：GPU memory / upload / delayed destroy 仍为 `Partial`；poll 粒度与限制已公开闭合，真实非阻塞 submission-index 查询和后台 worker 仍是未实现能力。

## 2026-05-19 本轮进展：非阻塞 submission-index retirement poll capability gate

- public `RendererFeature::NonblockingResourceRetirementPoll` 与 `RendererFeatures::NONBLOCKING_RESOURCE_RETIREMENT_POLL` 已加入统一 feature/capability 体系。
- 当前 headless/wgpu 路径均不声明该 capability；`feature_info()` 返回 `supported = false`、`implementation = ConfigGate`，reason 明确为当前 wgpu backend 使用 queue-empty fallback，尚不支持真正非阻塞 submission-index retirement polling。
- 该 gate 与 `BackendResourceRetirementStats::{nonblocking_submission_index_poll_supported, queue_empty_poll_fallback, last_poll_used_queue_empty_fallback}` 对齐，用户可同时从 feature API 和 stats/capture 观察能力边界。
- 验证：`renderer_feature`、`background_resource_retirement`、`renderer_memory_stats_expose_backend_tombstone_retirement` 相关测试通过。
- 状态：GPU memory / upload / delayed destroy 仍为 `Partial`；能力 gate 已闭合，真实非阻塞 per-submission 完成查询仍未实现。

## 2026-05-19 本轮进展：frame capture/tooling feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `FrameCapture`、`ExternalFrameCaptureHooks`、`NativeFrameDebuggerCapture`。
- `FrameCapture` 和 `ExternalFrameCaptureHooks` 在当前 facade/tooling 层声明 supported，分别对应 internal capture payload 与已存在的 registered external-hook handoff API。
- `NativeFrameDebuggerCapture` 显式 unsupported，reason 指向原生 RenderDoc/external debugger SDK 未链接；当前可用路径是注册外部 capture hook，而不是内置 SDK 调用。
- 该 gate 与 `FrameCaptureBackendInfo`、`FrameCaptureIntegration`、`capture_next_frame()` 的 hook-gated 错误路径对齐，capture/tooling 能力不再只隐藏在 backend info API 中。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer capture_options_validate_backend_hooks -- --nocapture` 通过。
- 状态：Frame API / stats / capture 仍为 `Partial`；内部 capture 与 external-hook handoff capability 已闭合，真实 RenderDoc/外部调试器 SDK 调用仍是外部阻塞/未实现项。

## 2026-05-19 本轮进展：debug draw / editor report feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `DebugDraw` 与 `EditorDebugReports`。
- 两项在当前 facade/tooling 层声明 supported，对应已存在的 debug draw command/output 与 `Renderer::frame_debug_report()` editor-facing summary 路径。
- 这些 tooling 能力现在进入统一 `RendererCaps::features`、`Renderer::supports_feature()`、`Renderer::feature_info()` 与 `Renderer::feature_audit()`，不再只作为散落的 public API 存在。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` 通过。
- 状态：Debug draw / editor API 继续保持 `Partial`；facade capability gate 已闭合，但更深 editor 集成、外部调试器 SDK 与 backend-specific tooling 行为仍未完整闭合。

## 2026-05-19 本轮进展：animation / deformation / LOD feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `SkeletalAnimation`、`MorphTargets`、`LodSelection`、`MotionVectors`、`BackendGpuDeformation`。
- `SkeletalAnimation`、`MorphTargets`、`LodSelection` 作为 supported facade-semantic capability，对应当前 skeleton instance、morph weights、LOD group 与 frame output 语义。
- `MotionVectors` 作为 supported graph-semantic capability，对应当前 motion-vector frame output / RHI observable path。
- `BackendGpuDeformation` 显式 unsupported，reason 说明 backend GPU skinning/morph deformation buffers 尚未实现；当前 deformation 输出仍是 renderer/RHI observable facade semantics。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer deformation -- --nocapture`、`cargo test -p engine_renderer lod -- --nocapture`、`cargo test -p engine_renderer motion_vector -- --nocapture` 通过。
- 状态：Animation / skinning / morph / LOD 仍为 `Partial`；facade/graph capability tracking 已闭合，backend-real GPU deformation 路径仍未实现。

## 2026-05-19 本轮进展：light / shadow / environment / IBL feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `Lights`、`ShadowMapping`、`EnvironmentIbl`、`BackendIblConvolution`。
- `Lights`、`ShadowMapping`、`EnvironmentIbl` 作为 supported graph-semantic capability，对应当前 retained light resources、shadow/environment frame outputs、environment graph import 和 facade-retained IBL bake observability。
- `BackendIblConvolution` 显式 unsupported，reason 说明 backend-real IBL/environment convolution 尚未实现；当前 environment bake 是 renderer-retained facade output。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer environment_ -- --nocapture`、`cargo test -p engine_renderer light -- --nocapture` 通过。
- 状态：Light / shadow / environment / IBL 仍为 `Partial`；facade/graph capability tracking 已闭合，backend-real convolution/capture path 仍未实现。

## 2026-05-19 本轮进展：RenderGraph / standard 3D pipeline feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `RenderGraph`、`CustomRenderGraphPasses`、`Standard3dPipeline`、`BackendRealStandard3dPipeline`。
- `RenderGraph` 作为 supported core graph-semantic capability，对应当前 graph builder、resource lifetime、RHI execution hook 与 validation 语义。
- `CustomRenderGraphPasses` 与 `Standard3dPipeline` 作为 supported graph-semantic capability，对应 custom graph extension 和 standard 3D graph/frame output 语义。
- `BackendRealStandard3dPipeline` 显式 unsupported，reason 说明完整 backend-real standard 3D pass execution 尚未闭合；当前标准管线仍混合 facade、RHI 与部分 backend-wgpu 执行证据。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer graph_ -- --nocapture`、`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` 通过。
- 状态：RenderGraph 基础能力继续保持可验；Standard 3D RenderGraph 仍为 `Partial`，因为全 backend-real standard pass execution 尚未实现。

## 2026-05-19 本轮进展：pipeline cache / shader variant cache feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `PipelineCache`、`ShaderVariantCache`、`CompleteBackendPipelineCache`。
- `PipelineCache` 与 `ShaderVariantCache` 作为 supported facade-semantic capability，对应 public pipeline warmup/cache stats/entry introspection 与 shader variant warmup/cache observability。
- `CompleteBackendPipelineCache` 显式 unsupported，reason 说明 complete backend-native pipeline cache coverage 尚未实现；当前 backend-wgpu reflected native cache 已有真实对象/统计，但 facade cache entries 仍可能缺 backend object，整体仍是 partial。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer pipeline -- --nocapture`、`cargo test -p engine_renderer shader -- --nocapture` 通过。
- 状态：Pipeline / pipeline key / cache 与 Shader variants 仍为 `Partial`；public facade/cache observability capability 已闭合，完整 backend-native cache coverage 仍未实现。

## 2026-05-19 本轮进展：upload / residency / streaming / delayed destroy feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `UploadQueue`、`ResourceResidency`、`StreamingResources`、`DelayedResourceDestroy`。
- 四项作为 supported facade-semantic capability，对应当前 upload stats/flush/submitted-frame bookkeeping、resource residency transitions、streaming memory/capture observability、frame-latency/submission-boundary delayed destroy semantics。
- 这些 memory/resource capabilities 现在进入统一 `RendererCaps::features`、`Renderer::supports_feature()`、`Renderer::feature_info()` 与 `Renderer::feature_audit()`。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer resource_residency_controls_streamed_meshes_and_textures -- --nocapture`、`cargo test -p engine_renderer submitted_frame -- --nocapture`、`cargo test -p engine_renderer poll_resource_retirements_completes_only_prior_submission_work -- --nocapture` 通过。
- 状态：GPU memory / upload / streaming 仍为 `Partial`；facade memory/resource capability tracking 与 cooperative background retirement startup/observability 已闭合，backend tombstone coverage 和 true nonblocking per-submission polling 仍未完整实现。

## 2026-05-19 本轮进展：基础 facade resource / scene / view feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `ResourceLifecycle`、`MeshResources`、`BufferResources`、`TextureResources`、`SamplerResources`、`MaterialSystem`、`RetainedScene`、`CameraViewRenderTargets`、`EcsExtractBoundary`。
- `ResourceLifecycle`、mesh/buffer/texture/sampler/material、retained scene、camera/view/render target 作为 supported core facade-semantic capability，对应当前 public resource create/update/destroy/status/info、scene command buffer、view/render target validation 语义。
- `EcsExtractBoundary` 作为 supported optional facade-semantic capability，对应当前 ECS-like extract fixture 到 retained scene/frame stats 的边界语义。
- 这些基础 facade capabilities 现在进入统一 `RendererCaps::features`、`Renderer::supports_feature()`、`Renderer::feature_info()` 与 `Renderer::feature_audit()`。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`generic_resource_lifecycle_covers_public_resource_kinds`、`custom_material_parameters_are_schema_validated`、`scene_command_buffer_rejects_destroyed_resource_handles_before_mutation`、`render_targets_are_validated_and_can_back_offscreen_views`、`ecs_like_extract_fixture_drives_scene_commands_and_frame_stats` 通过。
- 状态：基础 facade capability tracking 已闭合；backend-real execution、specialized stale-handle coverage 和完整 renderer 层闭环仍按矩阵继续保留未完成/Partial 项。

## 2026-05-19 本轮进展：native frame debugger capture unsupported error 对齐

- public `RendererFeature::NativeFrameDebuggerCapture` 已作为 reserved unsupported feature 暴露，reason 指向当前未链接 RenderDoc/external debugger 原生 SDK。
- `Renderer::capture_next_frame()` 现在在直接请求 `FrameCaptureBackend::RenderDoc` 或 `FrameCaptureBackend::ExternalDebugger` 且没有可用外部 hook / SDK 时，返回 `RendererError::UnsupportedFeature(RendererFeature::NativeFrameDebuggerCapture)`。
- 已注册外部 capture hook 的路径仍允许排队并在 frame finish 输出 `BackendHookRequested`、hook label、SDK name、request id、queued frame 和 capture latency。
- 该变更把 feature gate、backend info、用户可见错误和 capture 测试断言对齐；真实 RenderDoc SDK / external debugger SDK 调用仍是外部阻塞，不能计为完整 renderer 层实现。

验证：`cargo test -p engine_renderer capture_options_validate_backend_hooks -- --nocapture` passed，1 passed；`cargo test -p engine_renderer renderer_feature -- --nocapture` passed，4 passed。

## 2026-05-19 本轮进展：standard 3D backend-native pass 覆盖率观测

- `RenderGraphStats` 新增 backend-native standard pass 覆盖字段：`backend_native_standard_passes`、`backend_native_standard_pass_labels`、`backend_missing_standard_pass_labels`、`backend_real_standard_pipeline_complete`。
- facade/backend graph stats 合并时，现在会把 backend-wgpu native pass label 映射到标准 3D 语义 pass：当前可识别 `Neo Directional Shadow Pass -> shadow_csm`、`Neo Spot/Point Shadow Pass -> shadow_point_spot`、`Neo Forward Opaque Pass -> forward_opaque`。
- 未被 backend native pass 覆盖的 standard graph pass 会进入 `backend_missing_standard_pass_labels`，例如 `depth_prepass`、`present`、gbuffer/deferred/post 等仍可被 frame stats / capture / debug report 观察为未完整 backend-real 覆盖。
- 验证：`cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed，1 passed。
- 状态：这是 `BackendRealStandard3dPipeline` 缺口的可观测性收口；完整 backend-real standard 3D pipeline 仍未实现，`RendererFeature::BackendRealStandard3dPipeline` 继续保持 unsupported/config-gated。

## 2026-05-19 本轮进展：editor debug report 暴露 standard backend 覆盖字段

- `FrameDebugReport` 新增平铺字段：`backend_native_standard_passes`、`backend_native_standard_pass_labels`、`backend_missing_standard_pass_labels`、`backend_real_standard_pipeline_complete`。
- 这些字段直接镜像 `FrameStats.graph` 中的 standard 3D backend-native pass 覆盖状态，避免 editor/tooling 只读取 pass label 或 RHI label 时误判完整 backend-real standard pipeline。
- 验证：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- 状态：debug/editor 可观测性已增强；`BackendRealStandard3dPipeline` 仍未完成，gbuffer/deferred/post/present 等 backend-real pass 仍需继续实现。

## 2026-05-19 本轮进展：frame capture 暴露 standard backend 覆盖字段

- `FrameCapture` 新增平铺字段：`backend_native_standard_passes`、`backend_native_standard_pass_labels`、`backend_missing_standard_pass_labels`、`backend_real_standard_pipeline_complete`。
- capture payload 现在直接镜像 `FrameStats.graph` 的 standard 3D backend-native pass 覆盖状态，与 editor/debug report 和 graph stats 保持一致。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 状态：frame stats / capture / debug report 对 `BackendRealStandard3dPipeline` 缺口的观测面更完整；真实 backend-wgpu gbuffer/deferred/post/present pass 仍未实现。

## 2026-05-19 本轮进展：backend-wgpu present 覆盖映射

- standard 3D backend-native 覆盖统计现在把 backend-wgpu `Neo Forward Opaque Pass` 映射到 `forward_opaque`，并把 `Neo Transparent Pass` 映射到 `transparent` 与 `present`。
- 该映射表达当前 surface path 中 opaque draw 与 transparent/final output 已分离，最终 surface output/present 语义由 `Neo Transparent Pass` 承担，避免把已由 backend-wgpu 完成的 native output 误报为 missing standard pass。
- `backend_missing_standard_pass_labels` 仍会保留 `depth_prepass`、`gbuffer`、`deferred_lighting`、post process 等尚未被 backend-native pass 覆盖的标准 3D pass。
- 验证：`cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed，1 passed。
- 状态：`BackendRealStandard3dPipeline` 仍未完成；本轮只修正 surface output/present 覆盖观测。

## 2026-05-19 本轮进展：standard backend 覆盖计数字段

- `RenderGraphStats`、`FrameDebugReport` 与 `FrameCapture` 新增 `backend_total_standard_passes` 和 `backend_missing_standard_passes`。
- standard 3D backend-native 覆盖现在同时暴露总 standard pass 数、backend-native 覆盖数、缺失数、覆盖 label、缺失 label 和 complete bool，tooling/CI 不再需要解析 label 列表才能判断 `BackendRealStandard3dPipeline` 差距。
- 验证：`cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 状态：这是 standard 3D backend 覆盖缺口的观测面收口；真实 backend-wgpu gbuffer/deferred/post 等 pass 仍未实现，renderer goal 未完成。

## 2026-05-19 本轮进展：RHI standard pass 观测字段

- `RenderGraphStats` 新增 `rhi_standard_passes` 与 `rhi_standard_pass_labels`，在 `execute_on_rhi` 路径中记录哪些标准 3D pass 真正进入 RHI command execution。
- `FrameDebugReport` 与 `FrameCapture` 同步平铺这两个字段，使 editor/debug/capture 可以直接区分 RHI-executed standard pass 和 backend-wgpu native standard pass 覆盖。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed，覆盖 deferred standard graph 的 RHI standard pass labels。
- 验证：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed，覆盖 editor/debug report 镜像。
- 验证：`cargo test -p engine_renderer graph_ -- --nocapture` passed，34 passed，覆盖 graph/RHI 执行路径。
- 状态：RHI standard pass 观测面已增强；该字段不把 headless/RHI 结果等同为 backend-wgpu native standard pipeline 完成，`BackendRealStandard3dPipeline` 仍未完成。

## 2026-05-19 本轮进展：backend-wgpu native depth prepass

- `render_wgpu::MeshRenderer` 新增 fragment-less depth-only pipeline：`Neo Mesh Depth Prepass Pipeline` 与 `Neo Double-Sided Mesh Depth Prepass Pipeline`。
- backend-wgpu surface frame 现在在 `Neo Forward Opaque Pass` 前执行真实 `Neo Depth Prepass`：对 depth-enabled surface depth target clear/store，只绘制 visible opaque + depth_write batches，随后主 mesh pass load 已写入 depth。
- backend native pass label 顺序现在包含 `Neo Depth Prepass`，renderer-level standard coverage 将其映射为 `depth_prepass`。
- `backend_native_standard_pass_labels` 现在能把 `shadow_csm`、`depth_prepass`、`forward_opaque`、`present` 标为 backend-native 覆盖，减少 `BackendRealStandard3dPipeline` 的真实缺口。
- 验证：`cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 状态：这是 `BackendRealStandard3dPipeline` 的真实 backend-wgpu 增量；完整 standard 3D backend pipeline 仍未完成，`gbuffer`、`deferred_lighting`、post process 等仍缺 backend-native pass。

## 2026-05-19 本轮进展：native depth prepass stats / pipeline inventory 收口

- `MeshRenderStats` 新增 `mesh_pass_draw_call_count` 与 `depth_prepass_draw_call_count`，`draw_call_count` 现在包含主 mesh pass draw 与 native depth prepass draw 的总和。
- backend-wgpu frame stats 通过 `MeshRenderStats::draw_call_count` 暴露真实 native draw work，不再在新增 `Neo Depth Prepass` 后低报 draw calls。
- `MeshRenderer::STATIC_RENDER_PIPELINE_COUNT` 从 24 更新为 26，包含两个新增 fragment-less depth-prepass pipeline，pipeline cache/backend inventory 观测不再低报。
- 验证：`cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture` passed，1 passed。
- 验证：`cargo test -p render_wgpu mesh_renderer_static_pipeline_inventory_is_reported -- --nocapture` passed，integration target 1 passed。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 状态：native depth prepass 的 frame stats 与 pipeline inventory 观测已收口；完整 backend-real standard 3D pipeline 仍未完成，gbuffer/deferred/post 等 backend-native pass 仍缺。

## 2026-05-19 本轮进展：backend-wgpu transparent 覆盖映射

- standard 3D backend-native 覆盖统计现在把 backend-wgpu `Neo Transparent Pass` 映射到 `transparent`。
- 该映射基于 `render_wgpu::MeshRenderer` 已存在的 alpha-blend pipelines、`transparent_draw_call_count` 和同一 native mesh render pass 中的 alpha-blend draw 执行路径。
- coverage 现在可把 `shadow_csm`、`depth_prepass`、`forward_opaque`、`transparent`、`present` 标为 backend-native 覆盖；`gbuffer`、`deferred_lighting`、post process 等仍保持 missing。
- 验证：`cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture` passed，4 passed。
- 状态：这是 `BackendRealStandard3dPipeline` 的覆盖映射收口，不代表完整 renderer goal 完成。

## 2026-05-19 本轮进展：backend native draw breakdown 高层透传

- `FrameStats` 新增 backend native draw breakdown：`backend_mesh_pass_draw_calls`、`backend_depth_prepass_draw_calls`、`backend_opaque_draw_calls`、`backend_transparent_draw_calls`。
- `FrameDebugReport` 与 `FrameCapture` 同步平铺这些字段，editor/debug/capture 现在能区分主 mesh pass、native depth prepass、opaque draw 与 transparent draw。
- `frame_stats_from_wgpu_metrics` 从 `MeshRenderStats` 透传 draw breakdown，`draw_calls` 仍表示 backend native draw 总量。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 验证：`cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 状态：native depth/mesh pass 的高层观测面已收口；完整 backend-real standard 3D pipeline 仍未完成，gbuffer/deferred/post 等 backend-native pass 仍缺。

## 2026-05-19 本轮进展：native mesh pass opaque/transparent phase 顺序

- backend-wgpu surface path 现在拆成真实 native `Neo Forward Opaque Pass` 与 `Neo Transparent Pass`：opaque batches 在前者执行，alpha-blend transparent batches 在后者执行。
- 新增 `mesh_pass_phase_order` helper，防止 batch 输入顺序把 transparent draw 排到 opaque draw 之前。
- 该改动让 `Neo Forward Opaque Pass -> forward_opaque` and `Neo Transparent Pass -> transparent` 的 backend-native coverage 映射更接近标准 3D 管线语义；仍不是独立 transparent render pass。
- 验证：`cargo test -p render_wgpu mesh_pass_phase_order_draws_opaque_before_transparent -- --nocapture` passed，1 passed。
- 验证：`cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture` passed，1 passed。
- 验证：`cargo test -p render_wgpu mesh_renderer_static_pipeline_inventory_is_reported -- --nocapture` passed，integration target 1 passed。
- 状态：forward/transparent native mesh phase 顺序已收口；完整 backend-real standard 3D pipeline 仍未完成，gbuffer/deferred/post 等 backend-native pass 仍缺。

## 2026-05-19 本轮进展：backend-wgpu transparent back-to-front batch 排序

- backend-wgpu `Neo Transparent Pass` 的 transparent batches 现在按相机距离 back-to-front 排序，并在 `Neo Forward Opaque Pass` 之后执行。
- 对 instanced batch，排序距离使用 batch 内最远 instance 的 model-matrix translation 到相机位置的距离，避免拆分 instance batch 的大重构，同时比输入顺序更符合透明渲染语义。
- 新增纯 CPU helper 验证：opaque phase 排在 transparent 前、transparent batch 距离使用最远 instance。
- 验证：`cargo test -p render_wgpu mesh_pass_phase_order_draws_opaque_before_transparent -- --nocapture` passed，1 passed。
- 验证：`cargo test -p render_wgpu mesh_batch_distance_uses_farthest_instance_for_transparent_sorting -- --nocapture` passed，1 passed。
- 验证：`cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture` passed，1 passed。
- 状态：transparent native mesh-pass 语义更完整；这仍不是独立 transparent render pass，完整 backend-real standard 3D pipeline 仍未完成。

## 2026-05-19 本轮进展：native shadow draw breakdown 高层透传

- `MeshRenderStats` 新增 `shadow_draw_call_count`、`directional_shadow_draw_call_count`、`spot_shadow_draw_call_count`、`point_shadow_draw_call_count`。
- `MeshRenderStats::draw_call_count` 现在表示 backend native 总 draw work：shadow draw + depth prepass draw + mesh pass draw。
- `FrameStats`、`FrameDebugReport` 与 `FrameCapture` 同步新增 backend shadow draw breakdown 字段，native directional/spot/point shadow pass 的实际 draw work 不再只通过 pass label 间接可见。
- 验证：`cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 验证：`cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 状态：native shadow/depth/mesh draw observability 更完整；完整 backend-real standard 3D pipeline 仍未完成，gbuffer/deferred/post 等 backend-native pass 仍缺。

## 2026-05-19 本轮进展：backend native draw breakdown 进入 FrameProfile

- `FrameProfile` 新增 backend native draw breakdown 字段：mesh pass、depth prepass、shadow total、directional shadow、spot shadow、point shadow、opaque、transparent。
- profiling payload 现在和 `FrameStats`、`FrameCapture`、`FrameDebugReport` 一样能观察 backend native draw work，不再只暴露总 `draw_calls`。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 状态：profiling/tooling 观测面继续收口；完整 renderer goal 仍未完成，backend-native gbuffer/deferred/post 等 standard pass 仍缺。

## 2026-05-19 本轮进展：backend native pass draw 结构化快照

- 新增 `BackendNativePassDrawStats { pass_label, draw_calls }`，并在 `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 中暴露 `backend_native_pass_draws`。
- backend-wgpu metrics 现在把 native pass label 与 draw count 结构化绑定：directional shadow、spot shadow、point shadow、depth prepass、mesh pass。
- 该字段防止 tooling 只能分别读取 `rhi_executed_pass_labels` 和 draw breakdown 后自行推断 native pass work，也能避免 pass label 与 draw stats 漂移。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 验证：`cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 状态：backend native pass/draw observability 更完整；完整 renderer goal 仍未完成，gbuffer/deferred/post 等 backend-native standard pass 仍缺。

## 2026-05-19 本轮进展：backend native pass instance 计数

- `BackendNativePassDrawStats` 新增 `pass_instances`，用于表达同名 native pass label 的实际实例数量。
- backend-wgpu native pass draw 快照现在能区分 draw count 与 pass instance count，例如 directional shadow cascades 或 point shadow cube faces 会形成多个同名 native pass instance。
- 新增验证：`cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture` passed，1 passed。
- 回归验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 回归验证：`cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture` passed，1 passed。
- 状态：backend native pass/draw observability 更精确；完整 renderer goal 仍未完成，gbuffer/deferred/post 等 backend-native standard pass 仍缺。

## 2026-05-19 本轮进展：native skybox draw 统计透传

- `MeshRenderStats` 新增 `skybox_draw_call_count`，`draw_call_count` 现在包含 skybox draw + shadow draw + depth prepass draw + mesh batch draw。
- `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 新增 `backend_skybox_draw_calls`，backend-wgpu skybox draw 不再从高层观测面丢失。
- `BackendNativePassDrawStats` 现在分别报告 `Neo Forward Opaque Pass` 的 opaque mesh + skybox draw count，以及 `Neo Transparent Pass` 的 transparent draw count。
- 验证：`cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 验证：`cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 状态：native mesh/skybox/shadow/depth draw observability 更完整；完整 renderer goal 仍未完成，gbuffer/deferred/post 等 backend-native standard pass 仍缺。
## 2026-05-19 本轮进展：backend-wgpu forward/transparent native pass 拆分

- backend-wgpu surface frame 不再用单个 `Neo Mesh Pass` 同时承载 opaque、transparent 和 present 语义；现在真实创建 `Neo Forward Opaque Pass` 与 `Neo Transparent Pass` 两个 wgpu render pass。
- `Neo Forward Opaque Pass` 负责 clear color、load/clear depth、skybox draw 和 opaque mesh draw；MSAA 路径在该 pass 只 store 中间 color，不提前 resolve。
- `Neo Transparent Pass` load opaque pass 的 color/depth 并执行 alpha-blend transparent batches；`Neo Post Process Pass` 再 load color/depth，执行 post-pass hook，并在该最终 pass 处理 resolve/store 到 surface output。
- backend native standard pass coverage 更新为 `Neo Forward Opaque Pass -> forward_opaque`、`Neo Transparent Pass -> transparent/present`；不再通过旧 `Neo Mesh Pass` 映射多个标准 pass。
- `BackendNativePassDrawStats` 现在按真实 native pass 分别输出 `Neo Forward Opaque Pass` 和 `Neo Transparent Pass` 的 draw count，skybox draw 归属 forward opaque pass。
- 状态：forward opaque / transparent / present 的 backend-native pass 语义更接近标准 3D 管线；完整 renderer goal 仍未完成，gbuffer、deferred lighting、post-process pass family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。


### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，2 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：backend-wgpu actual native pass label 快照

- `render_wgpu::MeshRenderStats` 新增固定容量 actual native pass label 快照，保持 `Copy` 兼容，同时记录本帧真实进入 `begin_render_pass` 的 wgpu pass label 顺序。
- `MeshRenderer::render_batches_with_environment_probes_and_post_pass()` 现在在实际创建 directional/spot/point shadow、depth prepass、forward opaque 和 transparent render pass 时记录 label；这比按 scene/visible count 推导更接近真实 backend 行为。
- `engine_renderer` backend-wgpu frame stats 现在优先使用 `MeshRenderStats` 的 actual native pass labels；仅当旧路径或手写 stats 没有 label 快照时，才回退到 `default_wgpu_pass_labels()`。
- 新增测试覆盖 actual label 优先级和 fallback 行为，避免 graph/debug/capture 继续依赖预估 pass label 作为真实 backend 证据。
- 状态：backend native pass observability 更真实；完整 renderer goal 仍未完成，gbuffer、deferred lighting、post-process pass family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_prefer_actual_native_pass_labels_over_default_estimate -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：render_wgpu native pass label stats API 验证

- `render_wgpu` 新增底层 stats API 测试，直接覆盖 `MeshRenderStats::record_native_pass_label()` 与 `native_pass_label_strings()`，确认 actual native pass label 顺序能从 backend stats 中导出。
- 新增固定容量边界测试，确认 native pass label 快照超过容量时不会溢出或破坏 stats 结构；这保持 `MeshRenderStats` 的 `Copy` 兼容，同时提供 bounded observability。
- 该验证补齐了上一轮 engine_renderer 层测试的底层证据：上层不再只依赖手写 `MeshRenderStats` fixture，而有 render_wgpu stats API 自身的行为测试。
- 状态：backend native pass label observability 的底层 API 证据更完整；完整 renderer goal 仍未完成，gbuffer、deferred lighting、post-process pass family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_ -- --nocapture`: passed，3 passed。

## 2026-05-19 本轮进展：post-pass native draw 归入真实 native pass breakdown

- backend-wgpu queued native pipeline draws 原本已计入 `FrameStats::draw_calls`，但结构化 `BackendNativePassDrawStats` 只统计 material transparent draw，未把 post-pass native draws 归入独立的 `Neo Post Process Pass`。
- 新增 `record_native_post_pass_draws()`，统一更新总 draw call 和 `Neo Transparent Pass` 的 per-pass draw count，避免总数与 per-native-pass breakdown 不一致。
- 新增测试确认 native post-pass draw 会增加 `draw_calls`，并被合并到 `BackendNativePassDrawStats { pass_label: "Neo Post Process Pass" }`。
- 状态：post-pass/native custom draw 的 frame observability 更一致；完整 renderer goal 仍未完成，gbuffer、deferred lighting、post-process pass family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，3 passed。

## 2026-05-19 本轮进展：post-pass draw flat stats 暴露到 profile/debug/capture

- `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 新增 `backend_post_pass_draw_calls`，用于直接观察 queued native pipeline draw / post-pass custom draw 数量。
- backend-wgpu `record_native_post_pass_draws()` 现在同时更新总 `draw_calls`、flat `backend_post_pass_draw_calls` 与 `Neo Post Process Pass` 的 `BackendNativePassDrawStats`，让总量、平铺字段和 per-pass breakdown 保持一致。
- profile/debug/capture 映射测试已补断言，确认编辑器报告、profiling payload 和 capture payload 不丢该字段。
- 状态：post-pass/native custom draw 的 public observability 更完整；完整 renderer goal 仍未完成，gbuffer、deferred lighting、post-process pass family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，3 passed。
## 2026-05-19 本轮进展：backend-wgpu 独立 Neo Post Process Pass

- backend-wgpu surface path 现在从 `Neo Transparent Pass` 后拆出真实 `Neo Post Process Pass`：transparent pass 只负责 alpha-blend transparent batches 并 store 中间 color/depth，post-process pass 再 load color/depth、执行 post-pass hook，并在最终 pass 上 resolve/store 到 surface output。
- actual native pass label、`default_wgpu_pass_labels()`、frame stats、debug report 和 graph coverage 现在都包含 `Neo Post Process Pass`。
- standard backend-native coverage 新增 `post_process_resolve -> Neo Post Process Pass`，`present` 也改由最终 `Neo Post Process Pass` 覆盖；`post_process_resolve` 已加入 standard 3D pass label 识别。
- queued native post-pass draw 的 flat `backend_post_pass_draw_calls` 与 per-pass `BackendNativePassDrawStats` 现在归属 `Neo Post Process Pass`，不再混入 `Neo Transparent Pass`。
- 状态：post-process/custom native draw 具备独立 backend-native pass 语义和可观测输出；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。



## 2026-05-19 本轮进展：post-process native pass flat observability

- `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 新增 `backend_post_process_passes`，直接暴露本帧实际执行的 `Neo Post Process Pass` 实例数。
- `backend_post_process_passes` 与 `backend_post_pass_draw_calls` 分离：前者表达 native post-process pass 是否/执行几次，后者表达该 pass 中 queued native/custom draw 数量。
- backend-wgpu frame stats 由 actual/native pass labels 统计 `Neo Post Process Pass` 实例数，debug/profile/capture 映射测试已补断言，避免 editor/capture 只能从 label list 或 per-pass draw breakdown 间接推断。
- 状态：post-process pass 的 flat observability 更完整；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：native standard pass flat instance counters

- `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 新增 flat native pass instance counters：`backend_directional_shadow_passes`、`backend_spot_shadow_passes`、`backend_point_shadow_passes`、`backend_depth_prepass_passes`、`backend_forward_opaque_passes`、`backend_transparent_passes`。
- 这些字段与 draw-call counters 分离，表达真实 backend native pass 执行实例数；编辑器、profile 和 capture 不再必须解析 `rhi_executed_pass_labels` 或 `BackendNativePassDrawStats` 才能知道各类 native pass 是否执行。
- backend-wgpu stats 现在从 actual/native pass label 快照统计这些 pass instance counters，并继续保留 per-pass draw breakdown 用于 draw work 归因。
- 状态：standard pass observability 更平铺、更适合 editor/debug/capture 消费；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：native pass instance 与 draw-call 分离验证

- 新增 `wgpu_metrics_count_native_pass_instances_separately_from_draw_calls`，用重复 directional shadow cascade、spot shadow、point shadow cube faces、depth/forward/transparent/post passes 的同一帧 fixture 验证 flat pass instance counters 与 draw-call counters 分离。
- 该测试确认 `backend_directional_shadow_passes`、`backend_spot_shadow_passes`、`backend_point_shadow_passes`、`backend_depth_prepass_passes`、`backend_forward_opaque_passes`、`backend_transparent_passes`、`backend_post_process_passes` 只表达 native pass 实例数，不被 draw count 污染。
- 同一测试同时确认 shadow/depth/opaque/transparent/post draw counters 仍表达 draw work，避免 editor/debug/capture 把 repeated pass instance 和 draw workload 混为一类指标。
- 状态：native standard pass observability 的测试证据更完整；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_count_native_pass_instances_separately_from_draw_calls -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。

## 2026-05-19 本轮进展：native pass label 快照截断可观测

- `render_wgpu::MeshRenderStats` 新增 `native_pass_labels_dropped`，当 fixed-capacity actual native pass label 快照超过 `MAX_NATIVE_PASS_LABELS` 时记录被截断数量，不再静默丢失观测信息。
- `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 新增 `backend_native_pass_labels_dropped`，把 backend label 快照截断情况透传到 editor/debug/profile/capture。
- `mesh_render_stats_native_pass_labels_are_bounded` 现在验证超出容量时 dropped count 增加；`wgpu_metrics_count_native_pass_instances_separately_from_draw_calls` 验证 backend-wgpu stats 会保留该 dropped count。
- 状态：backend native pass label observability 的边界行为更明确；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：native pass label 快照容量可观测

- `render_wgpu::MeshRenderStats` 新增 `native_pass_label_capacity()`，显式暴露 actual native pass label 快照容量，避免上层依赖未导出的内部常量。
- `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 新增 `backend_native_pass_label_capacity`，与 `backend_native_pass_labels_dropped` 配套，让 editor/debug/profile/capture 能判断 native pass label 快照是否截断以及截断比例。
- backend-wgpu stats 现在从 `MeshRenderStats::native_pass_label_capacity()` 填充容量字段，避免 hard-code 容量或只暴露 dropped count。
- 状态：native pass label snapshot 的容量与截断行为均可观察；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：native pass label 截断时 graph pass count 修正

- backend-wgpu frame stats 现在在 actual native pass label 快照被截断时，将 `graph.pass_count` / `graph.rhi_executed_passes` 计算为 `recorded labels + backend_native_pass_labels_dropped`，避免只按保留下来的 label 数低估真实 native pass 执行次数。
- `graph.rhi_executed_pass_labels` 仍只保存未截断的 label 快照；`backend_native_pass_label_capacity` 与 `backend_native_pass_labels_dropped` 用于解释 label list 与 pass count 的差异。
- `wgpu_metrics_count_native_pass_instances_separately_from_draw_calls` 已补断言，覆盖 recorded label 数为 13、dropped 为 5 时 `rhi_executed_passes == pass_count == 18` 的边界。
- 状态：native pass label snapshot 截断时 graph stats 不再低估 pass 执行数量；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_count_native_pass_instances_separately_from_draw_calls -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。

## 2026-05-19 本轮进展：backend-wgpu 内建 identity fullscreen post-process draw

- `render_wgpu` 新增 `post_process.wgsl`，提供 fullscreen triangle vertex shader 与 alpha=0 fragment output；配合 alpha blending 形成不改变画面的 identity post-process draw，避免采样/写入同一 render target。
- `MeshRenderer` 新增 `Neo Post Process Color Pipeline` 与 `Neo Post Process Depth Pipeline`，按 post-process pass 是否带 depth attachment 选择；`STATIC_RENDER_PIPELINE_COUNT` 从 26 更新到 28。
- `Neo Post Process Pass` 现在默认执行一个真实 fullscreen draw，再执行 queued custom post-pass hook；`MeshRenderStats::post_process_draw_call_count`、`FrameStats::backend_post_process_draw_calls`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 均可观察该内建 draw。
- `BackendNativePassDrawStats { pass_label: "Neo Post Process Pass" }` 现在包含内建 post-process fullscreen draw，并会继续叠加 queued native/custom post-pass draw。
- 状态：post-process pass 不再只是空 pass 或 custom hook 容器，已有真实 backend-native fullscreen pipeline/draw；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_renderer_static_pipeline_inventory_is_reported -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：backend-wgpu fullscreen draw 映射到 RenderGraphStats

- backend-wgpu `frame_stats_from_wgpu_metrics()` 现在把 `MeshRenderStats::post_process_draw_call_count` 映射到 `RenderGraphStats::fullscreen_draws`，让 graph stats 也能观察内建 `Neo Post Process Pass` fullscreen work。
- 该字段与 `backend_post_process_draw_calls` 保持一致，但语义不同：`fullscreen_draws` 属于 graph/workload 视角，`backend_post_process_draw_calls` 属于 backend pass draw breakdown 视角。
- `wgpu_metrics_map_gpu_timestamps_to_frame_stats` 和 `wgpu_metrics_count_native_pass_instances_separately_from_draw_calls` 已补断言，确认 backend-native post-process fullscreen draw 不只出现在 draw breakdown，也进入 graph fullscreen draw 统计。
- 状态：post-process fullscreen work 的 graph observability 更完整；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。

## 2026-05-19 审计增量：GBuffer backend-native 闭合进度

- 已完成：backend-wgpu surface path 新增真实 `Neo GBuffer Pass`，使用 transient offscreen render target 承载 opaque batch draw，并进入 actual native pass label snapshot。
- 已完成：backend-wgpu stats 将 GBuffer pass/draw 映射到 `FrameStats`、`FrameProfile`、`FrameDebugReport`、`FrameCapture` 和 `BackendNativePassDrawStats`。
- 已完成：facade/backend graph merge 将 standard `gbuffer` 计入 backend native standard pass coverage，不再只能从 headless RHI label 证明该 pass 存在。
- 验证结果：本轮相关 9 条精确测试全部通过，覆盖 stats 映射、label 顺序、draw breakdown、debug report、facade/backend graph merge 和 frame profile/capture 透传。
- 仍未完成：该 GBuffer pass 目前是单 transient color target，不是完整 MRT GBuffer；deferred lighting 仍未读取 GBuffer；post-process family、pipeline cache、RenderDoc SDK、更多 backend resource lifetime/tombstone 仍是 renderer goal 阻塞项。
- 结论：完整 renderer goal 继续保持未完成；本轮只关闭 `gbuffer` 的 backend-native pass/observability 子缺口。

## 2026-05-19 审计增量：GBuffer MRT 与 shader validation

- 已完成：`Neo GBuffer Pass` 从单 color target 升级为 albedo/normal/material 三目标 MRT backend pass。
- 已完成：新增 `gbuffer.wgsl` 和 GBuffer 单面/双面、带 depth/不带 depth pipeline；pipeline inventory 从 28 更新到 32，并由测试覆盖。
- 已完成：修复 `post_process.wgsl` 的 naga shader validation 问题；真实 `render_smoke.exe` hidden launch 通过，说明当前 wgpu 设备路径可以创建新增/既有 shader pipelines。
- 验证结果：本轮相关 Rust 单元测试、pipeline inventory 测试、`cargo build --bin render_smoke` 和 hidden smoke launch 全部通过。
- 仍未完成：MRT GBuffer 还未进入 deferred lighting 数据流，仍缺 GBuffer sampling/lighting resolve/resource export；完整 renderer goal 不能关闭。

## 2026-05-19 audit update: backend-wgpu deferred lighting sampling pass

Completed in this slice:

- Added a real `Neo Deferred Lighting Pass` in `render_wgpu::MeshRenderer`.
- Converted GBuffer MRT outputs to sampleable resolved textures so a later pass can read them.
- Added `deferred_lighting.wgsl` sampling for albedo, normal, and material GBuffer textures.
- Added native pass labels, draw-call stats, pass-count stats, fullscreen draw accounting, debug/profile/capture mirroring, and semantic standard pass coverage for `deferred_lighting`.
- Increased static backend pipeline inventory from 32 to 33.

Validation performed:

- `cargo test -p render_wgpu deferred_lighting_shader_samples_gbuffer_mrt -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_map_gpu_timestamps_to_frame_stats -- --nocapture`
- `cargo test -p render_wgpu mesh_render_stats_ -- --nocapture`
- `cargo test -p render_wgpu mesh_renderer_static_pipeline_inventory_is_reported -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`
- `cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process closed with exit code 0 and empty stdout/stderr.

Remaining audit gap: deferred lighting is now real backend work, but full renderer-layer completion still requires making the lit target part of the authoritative frame output/post-process path, plus the remaining renderer_goal.md gaps around resource export/import, full post-process family, pipeline cache behavior, external capture SDK hooks, and other Partial/Stub/Missing matrix items.

Additional validation performed after the audit note above:

- `cargo test -p render_wgpu mesh_renderer_reports_static_pipeline_inventory -- --nocapture`

## 2026-05-19 audit update: deferred lighting consumed by final post-process

Completed after the deferred lighting sampling pass:

- Added `post_process_sampled.wgsl` for sampling `Neo Deferred Lighting Texture`.
- Added a sampled post-process pipeline and bind group path in `render_wgpu::MeshRenderer`.
- `Neo Post Process Pass` now samples the deferred lighting output when it exists and blends it into the final surface, preserving forward-rendered skybox/transparent pixels outside GBuffer coverage.
- Removed the unused depth variant of the post-process pipeline; static backend pipeline inventory remains 33 after replacing it with the sampled post-process pipeline.
- Fixed real wgpu validation by explicitly binding the shadow bind group in `Neo Depth Prepass` for the shared mesh pipeline layout.

Validation performed for this update:

- `cargo test -p render_wgpu sampled_post_process_shader_samples_deferred_lighting_target -- --nocapture`
- `cargo test -p render_wgpu mesh_render_stats_ -- --nocapture`
- `cargo test -p render_wgpu mesh_renderer_reports_static_pipeline_inventory -- --nocapture`
- `cargo test -p render_wgpu mesh_renderer_static_pipeline_inventory_is_reported -- --nocapture`
- `cargo test -p render_wgpu deferred_lighting_shader_samples_gbuffer_mrt -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo test -p engine_renderer wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds after the bind group fix; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

Remaining audit gap: this closes the minimal backend-wgpu GBuffer/deferred/post-process/surface path, but the full renderer-layer goal is still open because the remaining matrix still contains resource export/import, full post-process family, pipeline cache behavior, external capture SDK hooks, and other Partial/Stub/Missing items.

## 2026-05-19 audit update: tonemap native pass label and coverage

Completed in this slice:

- Renamed the sampled deferred-lighting final pass to `Neo Tonemap Post Process Pass` when the pass samples `Neo Deferred Lighting Texture`.
- Kept the fallback no-deferred-source path labeled `Neo Post Process Pass`.
- Updated backend-wgpu frame stats so post-process pass counts accept both labels.
- Updated backend native pass draw breakdown so `Neo Tonemap Post Process Pass` is reported separately from plain `Neo Post Process Pass`.
- Updated facade/backend standard pass coverage so semantic `tonemap` is covered only by `Neo Tonemap Post Process Pass`; `post_process_resolve` and `present` are covered by either post-process label.

Validation performed:

- `cargo test -p render_wgpu mesh_render_stats_ -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`
- `cargo test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`
- `cargo test -p render_wgpu sampled_post_process_shader_samples_deferred_lighting_target -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

Remaining audit gap: tonemap has a minimal real backend-native sampled path, but the full renderer-layer goal remains open because other post-process stages such as bloom, FXAA/TAA integration into backend output, color grading, SSR, depth of field, and motion blur are still not all backend-wgpu shader-complete paths.

## 2026-05-19 audit update: FXAA facade option and backend-native label

Completed in this slice:

- Added a facade-to-backend FXAA option path through `ViewQualitySettings::fxaa` and `WgpuPostProcessOptions`.
- Updated the sampled backend-wgpu post-process path so `post_process_sampled.wgsl` can run a simple FXAA filter before tonemap/gamma output.
- Exposed FXAA execution through the native pass label `Neo Fxaa Tonemap Post Process Pass`.
- Updated backend/facade standard pass coverage so semantic `fxaa` is only covered by the FXAA native label.
- Updated backend native pass draw breakdown tests to treat `Neo Fxaa Tonemap Post Process Pass` as a post-process draw-bearing native pass.

Remaining audit gap: FXAA now has a minimal real backend-wgpu sampled path and observability label, but the complete renderer-layer post-process goal remains open because bloom, TAA output integration, color grading LUT, SSR, depth of field, and motion blur are not all backend-wgpu shader-complete paths.

Validation performed:

- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`
- `cargo test -p render_wgpu sampled_post_process_shader_samples_deferred_lighting_target -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

## 2026-05-19 audit update: Bloom facade option and backend-native label

Completed in this slice:

- Added a facade-to-backend bloom option path through `ViewQualitySettings::bloom` and `WgpuPostProcessOptions`.
- Updated `post_process_sampled.wgsl` so the sampled backend-wgpu post-process path can add a small HDR bright-neighbor bloom contribution before tonemap/gamma output.
- Exposed bloom execution through native pass labels `Neo Bloom Tonemap Post Process Pass` and `Neo Bloom Fxaa Tonemap Post Process Pass`.
- Updated backend/facade standard pass coverage so semantic `bloom` is only covered by bloom native labels.
- Updated backend native pass draw breakdown tests to treat combined bloom/FXAA/tonemap labels as post-process draw-bearing native passes.

Validation performed:

- `cargo test -p render_wgpu sampled_post_process -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

Remaining audit gap: bloom now has a minimal single-pass backend-wgpu sampled path and observability label, but production bloom is not complete. The renderer still lacks a multi-resolution threshold/downsample/blur/upsample chain, separate bloom output resources, artist-facing bloom parameters, and full integration with the remaining post-process family.

## 2026-05-19 audit update: Color grading facade option and backend-native label

Completed in this slice:

- Added a facade-to-backend color grading option path from `ColorGradingMode::Lut` through `WgpuPostProcessOptions`.
- Expanded the sampled post-process uniform to carry a color grading flag.
- Updated `post_process_sampled.wgsl` so the sampled backend-wgpu post-process path can apply a small post-tonemap color grading curve before gamma output.
- Exposed color grading execution through native pass labels containing `Color Grading`, including combined bloom/FXAA/tonemap labels.
- Updated backend/facade standard pass coverage so semantic `color_grading` is only covered by color-grading native labels.
- Updated backend native pass draw breakdown tests to treat combined bloom/FXAA/tonemap/color-grading labels as post-process draw-bearing native passes.

Validation performed:

- `cargo test -p render_wgpu sampled_post_process -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

Remaining audit gap: color grading now has a minimal backend-wgpu sampled curve and observability label, but the documented LUT path is not complete. The renderer still lacks user-supplied 3D LUT texture resources, LUT lifecycle/validation, LUT sampling in the shader, and artist-facing grading controls.

## 2026-05-19 audit update: remaining post-process flags backend-visible sampled paths

Completed in this slice:

- Added facade-to-backend option fields for `taa`, `motion_blur`, `ssr`, and `depth_of_field` through `WgpuPostProcessOptions`.
- Expanded the sampled post-process uniform with `effect_flags` for those four effects.
- Updated `post_process_sampled.wgsl` with small single-pass sampled branches for TAA-like resolve, motion blur, SSR-like reflection tint, and depth-of-field-like radial blur.
- Replaced fixed post-process label combinations with dynamic native label generation in `render_wgpu`.
- Changed `MeshRenderStats` native pass label storage from static string snapshots to owned `String` snapshots so dynamic backend labels are preserved safely.
- Updated backend-wgpu native pass draw breakdown to recognize dynamic `Tonemap ... Post Process Pass` labels.
- Updated facade/backend standard pass coverage so semantic `taa`, `motion_blur`, `ssr`, and `depth_of_field` can be covered by backend-native post-process labels.

Validation performed:

- `cargo test -p render_wgpu sampled_post_process -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

Remaining audit gap: these are backend-visible sampled branches, not production implementations. TAA still lacks history/reprojection/jitter, motion blur lacks velocity-buffer blur, SSR lacks depth/normal ray marching, and depth of field lacks CoC plus blur-chain resources.

## 2026-05-19 audit update: SSAO backend-visible sampled path

Completed in this slice:

- Added a facade-to-backend `ssao` option field through `WgpuPostProcessOptions`.
- Expanded the sampled post-process uniform with `screen_space_flags.x` for SSAO.
- Updated `post_process_sampled.wgsl` with a small local-contrast ambient-occlusion-style sampled darkening branch.
- Updated dynamic post-process native labels so enabled SSAO appears as the `Ssao` token.
- Updated facade/backend standard pass coverage so semantic `ssao` can be covered by backend-native post-process labels.

Validation performed:

- `cargo test -p render_wgpu sampled_post_process -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

Remaining audit gap: SSAO is backend-visible, but not production-complete. The renderer still lacks a real depth/normal AO pass, AO blur, integration into deferred lighting, and public AO tuning parameters.

## 2026-05-19 audit update: HDR backend-visible post-process mode

Completed in this slice:

- Added a facade-to-backend `hdr` option field through `WgpuPostProcessOptions`.
- Reused `screen_space_flags.y` in the sampled post-process uniform for HDR mode.
- Updated `post_process_sampled.wgsl` with a small HDR exposure step before the remaining sampled post-process effects and tonemap/gamma output.
- Updated dynamic post-process native labels so enabled HDR appears as the `Hdr` token.
- Added `hdr` to standard pass label recognition and backend/facade coverage when a facade graph labels HDR as a standard capability.

Validation performed:

- `cargo test -p render_wgpu sampled_post_process -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

Remaining audit gap: HDR is backend-visible as a post-process mode, but the renderer still lacks complete HDR display support: HDR swapchain negotiation, render-target format validation/policy, exposure and white-point controls, and full display mapping.

## 2026-05-19 audit update: external frame capture hook callback handoff

Completed in this slice:

- Added public `FrameCaptureHookEvent` for external capture callback payloads.
- Added `Renderer::register_frame_capture_backend_callback` for registering a real callable hook alongside existing hook metadata.
- Frame finish now invokes the registered callback when a queued `RenderDoc` or `ExternalDebugger` capture reaches `FrameCaptureStatus::BackendHookRequested`.
- The callback payload includes backend, request id, capture label, queued frame index, completed frame index, resource-dump/open flags, hook label, and hook SDK name.
- `unregister_frame_capture_backend_hook` clears both metadata and callback state; metadata-only registration remains supported for existing callers.
- Capture tests now verify the callback event is actually delivered.

Validation performed:

- `cargo test -p engine_renderer capture_options_validate_backend_hooks -- --nocapture`
- `cargo test -p engine_renderer capture -- --nocapture`

Remaining audit gap: this closes the user-provided callback handoff path, but it is still not a built-in RenderDoc SDK integration. Native SDK loading and capture begin/end calls remain outside the engine unless supplied by the registered callback.

## 2026-05-19 audit update: external capture callback invocation observability

Completed in this slice:

- `FrameCapture` now records `external_hook_callback_invoked`.
- `FrameDebugReport` mirrors the value as `capture_external_hook_callback_invoked`.
- Capture tests distinguish metadata-only handoff from real callback invocation.
- Editor/debug reports can now observe callback invocation directly instead of inferring it from external side effects.

Validation performed:

- `cargo test -p engine_renderer capture -- --nocapture`
- `cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`

Remaining audit gap: callback invocation is observable, but native RenderDoc/external-debugger SDK work is still delegated to the registered callback.

## 2026-05-19 audit update: external capture callback failure observability

Completed in this slice:

- External frame capture callback invocation is now panic-safe at the renderer boundary.
- Callback panic payloads are converted into `FrameCaptureStatus::BackendHookFailed` instead of escaping `Frame::finish()`.
- `FrameCapture` records callback failure state and message through `external_hook_callback_failed` / `external_hook_callback_failure`.
- `FrameDebugReport` mirrors those fields for editor/debug tooling.
- Capture tests now cover metadata-only hook handoff, successful callable hook event delivery, panic callback failure reporting, and removed-hook fallback to `BackendUnavailable`.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer capture -- --nocapture`: passed，2 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。

Remaining audit gap: this makes user-provided external capture hooks observable and failure-safe, but native RenderDoc/external debugger SDK loading and capture begin/end calls remain outside the engine unless supplied by the registered callback. Complete renderer goal remains open.

## 2026-05-19 audit update: RenderGraph imported resource observability

Completed in this slice:

- `RenderGraphStats` now reports imported texture count, imported buffer count, imported texture labels, and imported buffer labels.
- Imported resource labels are sorted for deterministic debug/capture output.
- Existing import compile/RHI tests now prove imported resources are counted separately from transient graph resources.
- Frame/debug report validation confirms the expanded graph stats shape remains available through frame-level tooling.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。

Remaining audit gap: this closes only imported resource observability. It does not implement full graph resource export, cross-frame exported transient lifetime, or backend-wgpu graph resource import/export execution. Complete renderer goal remains open.

## 2026-05-19 audit update: RenderGraph resource export markers and stats

Completed in this slice:

- `RenderGraphBuilder` can now mark graph texture/buffer outputs with `export_texture()` and `export_buffer()`.
- `RenderGraphStats` reports exported texture/buffer counts and deterministic exported labels.
- Graph validation rejects export markers that point at missing graph resources.
- Existing import tests now cover imported resources that are re-exported, proving import/export counts are separated from transient resource counts.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。

Remaining audit gap: export markers are now public and observable, but complete resource export is not done until exported transients can become durable facade/backend resources or backend-wgpu graph outputs with clear lifetime semantics. Complete renderer goal remains open.

### 2026-05-19 audit supplement: RenderGraph export lifetime semantics

- Export markers now affect compiled lifetime data, extending exported resources through the final graph pass.
- Empty graph export is invalid and covered by the graph import/export test.
- Remaining gap: exported resources are not yet materialized as durable public renderer resources or backend-wgpu graph outputs.

## 2026-05-19 audit update: RenderGraph compiled export list

Completed in this slice:

- `CompiledRenderGraph` now carries `resource_exports` as structured compile output.
- `CompiledResourceExport` records exported graph resource identity and label with deterministic ordering.
- Crate-root export is available for extension/backend callers, and the prelude boundary remains clean.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer prelude_keeps_graph_and_rhi_types_out_of_game_layer_imports -- --nocapture`: passed，1 passed。

Remaining audit gap: export is now part of the compile artifact, but durable facade resource export and backend-wgpu graph output integration remain incomplete. Complete renderer goal remains open.

## 2026-05-19 audit update: RenderGraph RHI execution exports

Completed in this slice:

- RHI graph execution can now return materialized exports through `RhiGraphExecution`.
- Exported texture/buffer records include graph ids, stable labels, and actual RHI handles.
- Existing stats-only execution APIs remain compatible.
- The import/RHI test now exports imported texture/buffer resources and verifies the returned handles match the supplied RHI imports.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer prelude_keeps_graph_and_rhi_types_out_of_game_layer_imports -- --nocapture`: passed，1 passed。

Remaining audit gap: RHI export results are available to advanced callers, but there is still no durable public renderer resource promotion and no backend-wgpu graph/surface export integration. Complete renderer goal remains open.

### 2026-05-19 audit supplement: RHI transient resource exports

- Added coverage for exported transient graph texture/buffer resources through `execute_on_rhi_with_exports()`.
- The test verifies the resources are actually materialized by the RHI device and returned in `RhiResourceExports`.
- Remaining gap: exported transient resources are still frame-local RHI handles until a facade/backend promotion API is added.

## 2026-05-19 audit update: RenderGraph import/export stats aggregation

Completed in this slice:

- Frame-level graph stats accumulation now carries imported/exported resource counts and labels.
- Added a direct regression test for `accumulate_graph_stats()` so future graph stats fields are less likely to be silently dropped during multi-view or extension aggregation.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer accumulate_graph_stats_preserves_import_export_resource_observability -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。

Remaining audit gap: aggregation is fixed, but full renderer completion still requires durable public resource export and backend-wgpu graph/surface integration.

## 2026-05-19 audit update: facade/backend graph import-export merge

Completed in this slice:

- Facade/backend graph stats merge now preserves imported/exported resource observability fields.
- The backend-native standard pass merge regression test now also covers import/export counts and labels from both facade and backend stats.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer accumulate_graph_stats_preserves_import_export_resource_observability -- --nocapture`: passed，1 passed。

Remaining audit gap: merge support is ready, but backend-wgpu graph/surface export production and durable public resource promotion remain incomplete.

## 2026-05-19 audit update: RenderGraph export label validation

Completed in this slice:

- Export labels are now validated as public identifiers, not just debug text.
- Empty export labels and duplicate export labels fail graph compilation with `RendererError::RenderGraphValidation`.
- The builder import/export test now covers these error paths.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_exports_transient_resources -- --nocapture`: passed，1 passed。

Remaining audit gap: stable export labels are a prerequisite for lookup/promotion, but public durable resource promotion and backend-wgpu graph/surface export integration are still open.

## 2026-05-19 audit update: RHI export label lookup

Completed in this slice:

- RHI graph execution exports now have direct label lookup methods for both full export records and raw RHI handles.
- The transient export test covers successful texture/buffer lookup and missing-label `None` behavior.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_exports_transient_resources -- --nocapture`: passed，1 passed。

Remaining audit gap: exported RHI resources can be found by label, but they are not yet promoted to persistent public renderer handles and backend-wgpu surface execution still does not produce graph exports.

## 2026-05-19 audit update: RenderGraph extension export observability through facade

Completed in this slice:

- Added a facade-level test where a `RenderGraphExtension` exports transient texture and buffer outputs.
- The normal renderer frame path reports those exports in `FrameStats.graph`.
- `Renderer::frame_debug_report()` mirrors the same export counts and labels through `FrameDebugReport.graph`.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_exports_are_visible_in_frame_stats_and_debug_report -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_exports_transient_resources -- --nocapture`: passed，1 passed。

Remaining audit gap: graph extension exports are observable through the facade, but durable public resource promotion and backend-wgpu graph/surface export handles remain open.

### 2026-05-19 audit supplement: RenderGraph extension exports in frame capture

- The facade export observability test now covers frame capture payloads.
- `FrameCapture.graph` preserves exported resource counts and labels for public graph extension exports.

## 2026-05-19 audit update: profiled facade RenderGraph exports

Completed in this slice:

- Added coverage for graph extension exports while GPU profiling is enabled.
- The test proves the profiled headless-RHI facade path preserves export observability in stats, debug report, and capture payloads.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer profiled_render_graph_extension_exports_remain_visible_in_frame_outputs -- --nocapture`: passed，1 passed。

Remaining audit gap: backend-wgpu surface graph export handles and durable public resource promotion remain incomplete.

### 2026-05-19 audit supplement: graph/export regression suite

- Ran the focused `graph_` test suite after the graph export changes.
- Result: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_ -- --nocapture` passed，38 passed。

## 2026-05-19 audit update: RenderGraph resource label summaries

Completed in this slice:

- Added structured helper methods on `RenderGraphStats` for import/export label consumption.
- Added `RenderGraphResourceLabels` and re-exported it from the crate root.
- Tests cover both direct graph compile output and normal facade frame export observability using the new helper API.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_exports_are_visible_in_frame_stats_and_debug_report -- --nocapture`: passed，1 passed。

Remaining audit gap: structured labels reduce parsing friction, but exported graph outputs are not yet durable public renderer resources and backend-wgpu surface graph exports remain incomplete.

补充验证：`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer prelude_keeps_graph_and_rhi_types_out_of_game_layer_imports -- --nocapture` passed，1 passed，确认新增 graph resource label summary 类型保持在 crate-root API，不进入 game-layer prelude。

## 2026-05-19 audit update: RenderGraph export validation through facade

Completed in this slice:

- Added facade-level error-path coverage for duplicate graph export labels.
- The test proves graph export validation is visible through normal renderer frame usage, not only direct builder compilation.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_rejects_duplicate_export_labels_through_facade -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture`: passed，1 passed。

Remaining audit gap: complete renderer goal remains open; backend-wgpu graph/surface exports and durable public resource promotion are still missing.

## 2026-05-19 audit update: wgpu RHI RenderGraph exports

Completed in this slice:

- Added direct wgpu RHI coverage for `execute_on_rhi_with_exports()`.
- The test proves transient graph exports are materialized and returned on `WgpuRhiDevice`.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_wgpu_exports_transient_resources -- --nocapture`: passed，1 passed。

Remaining audit gap: direct backend-wgpu RHI graph export is covered, but backend-wgpu surface/standard-frame integration and durable public resource promotion remain incomplete.

## 2026-05-19 audit update: pipeline cache backend-object coverage helpers

Completed in this slice:

- Added public helper methods on `PipelineCacheStats` for backend-object coverage and gap checks.
- Extended the pipeline warmup test to assert facade-ready entries without backend objects are reported through the helpers.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer pipeline_warmup_validates_pipeline_keys -- --nocapture`: passed，1 passed。

Remaining audit gap: helpers make the gap explicit, but complete backend-native pipeline cache coverage remains incomplete.

## 2026-05-19 audit update: pipeline cache merge coverage semantics

Completed in this slice:

- Extended pipeline cache merge testing to cover backend-object coverage helper semantics.
- Confirmed backend inventory counts do not hide facade-ready entries without backend objects.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer pipeline_cache_stats_merge_preserves_facade_counts_and_backend_inventory -- --nocapture`: passed，1 passed。

Remaining audit gap: backend-native pipeline cache coverage remains partial; the current work improves truthful observability.

## 2026-05-19 audit update: backend-wgpu pipeline cache coverage merge semantics

Completed in this slice:

- Extended backend-wgpu pipeline cache merge regression coverage for backend-object gap helpers.
- Confirmed native backend inventory is not mistaken for complete facade/backend cache coverage.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory -- --nocapture`: passed，1 passed。

Remaining audit gap: complete backend-native pipeline cache coverage remains partial; observability is now stricter.

## 2026-05-19 audit update: pipeline cache coverage helpers in debug/capture outputs

Completed in this slice:

- Extended frame debug report coverage to assert pipeline cache backend-object helper semantics are preserved in debug and capture payloads.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。

Remaining audit gap: this is observability coverage; complete backend-native pipeline cache implementation is still incomplete.

## 2026-05-19 - Pipeline cache backend-object coverage audit

- Implemented explicit cache coverage helpers for facade ready/used entries versus backend/native pipeline objects.
- Confirmed debug report and capture payloads retain those helper semantics for editor diagnostics.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed: 1 test.

## 2026-05-19 audit update: public RenderGraph export promotion

- Added `Renderer::execute_graph_to_resources`, a public facade path that executes `RenderGraphBuilder` through RHI and promotes exported transient resources to durable renderer handles.
- `RhiTextureExport` / `RhiBufferExport` now carry graph descriptors for transient exports, and exported transient resources get RHI `COPY_SRC` usage so promotion can perform real RHI readback instead of reporting labels only.
- `RendererGraphExecution` and `RendererGraphResourceExports` expose label lookup for promoted `TextureHandle` / `BufferHandle` values. Imported renderer resources exported by a graph resolve to their original public handles and are marked `promoted: false`.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_exported_transients_to_public_handles -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_ -- --nocapture` passed: 3 tests.

Remaining audit gap: explicit public RHI/headless graph export promotion is now real, but backend-wgpu surface/standard-frame graph exports and full renderer-layer completion remain open.

## 2026-05-19 audit update: public graph imported resource data flow

- Public `Renderer::execute_graph_to_resources` now uploads imported public resource contents into RHI import resources before graph execution.
- Exported imported renderer resources are written back into the original public handle instead of only returning the handle label, making graph-side writes observable through public `buffer_bytes` / `texture_bytes`.
- Added regression coverage where the pass reads imported texture/buffer contents through RHI, writes new data, exports both imported resources, and verifies the original public handles carry the updated data with `promoted: false`.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_uploads_and_writes_back_imported_exports -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_exported_transients_to_public_handles -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_ -- --nocapture` passed: 3 tests.

Remaining audit gap: public RHI/headless graph import/export data flow is now real for covered resources, but full backend-wgpu surface/standard-frame export production and broader texture shape/format writeback remain incomplete.

## 2026-05-19 audit update: Depth32Float graph writeback

- Added `RhiDevice::write_texture_depth32f` with headless and backend-wgpu RHI implementations.
- Public graph imported depth textures now upload their public bytes before execution and write exported RHI results back to the original `TextureHandle`.
- Added regression coverage that reads the imported depth values inside the graph pass, writes new depth values through RHI, exports the imported texture, and verifies the original public texture bytes changed while preserving handle identity.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_uploads_and_writes_back_depth_imports -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture` passed: 3 tests. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_ -- --nocapture` passed: 3 tests.

Remaining audit gap: this closes Depth32Float for the explicit public RHI/headless graph path, not the complete array/cube/mip/MSAA texture writeback set or backend-wgpu surface/standard-frame exported frame outputs.

## 2026-05-19 audit update: 8-bit sRGB/BGRA graph export promotion

- RHI raw 8-bit texture read/write now accepts `Rgba8UnormSrgb` and `Bgra8UnormSrgb` in addition to `Rgba8Unorm`.
- Public `Renderer::execute_graph_to_resources` now promotes exported transient sRGB/BGRA graph textures into durable public textures rather than rejecting them as unsupported readback formats.
- Added regression coverage that writes transient sRGB and BGRA graph textures through RHI, exports both, and verifies promoted public handles preserve format and byte contents.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_8bit_srgb_and_bgra_exports -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture` passed: 4 tests. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_ -- --nocapture` passed: 3 tests.

Remaining audit gap: explicit public graph promotion is broader, but full renderer completion still requires array/cube/3D/mip/MSAA texture data paths and backend-wgpu surface/standard-frame graph exports.

## 2026-05-19 audit update: graph texture-shape unsupported path

- Public graph imported texture upload/writeback now explicitly gates unsupported texture shapes: mipmapped, array, and MSAA imports fail with `RenderGraphValidation`.
- Added regression coverage for mipped 2D, 2D array, and MSAA imported texture descriptors, plus a full `execute_graph_to_resources_` regression pass for supported paths.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_rejects_unsupported_imported_texture_shapes -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture` passed: 5 tests.

Remaining audit gap: this is a tested unsupported path, not full mip/array/cube/3D/MSAA graph texture data implementation. Complete renderer goal remains open.

## 2026-05-19 audit update: public graph export query surface

- Renderer facade now records the last successful public graph execution and exposes promoted export handles via `last_graph_execution` and `last_graph_resource_exports`.
- A failed `execute_graph_to_resources` call clears that cached result, so observers do not see stale graph export handles after validation or execution errors.
- Added regression coverage for successful buffer export query and stale-result clearing after an unsupported imported texture shape fails.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture` passed: 5 tests.

Remaining audit gap: explicit public graph export handles are queryable, but standard frame/capture/debug graph export integration and backend-wgpu surface graph output production remain open.

## 2026-05-19 audit update: frame/capture public graph export observability

- `FrameDebugReport` and `FrameCapture` now carry the latest successful explicit public graph execution result, including promoted public resource export handles.
- Added regression coverage that executes a public graph exporting a buffer, renders a captured frame, and verifies both debug report and capture payload can resolve the exported public buffer handle by label.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_ -- --nocapture` passed: 3 tests. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture` passed: 5 tests.

Remaining audit gap: this closes frame/debug/capture observability for explicit public graph exports, but it is not backend-wgpu surface/standard-frame graph export production as durable public frame outputs. Complete renderer goal remains open.

## 2026-05-19 audit update: FrameStats public graph export observability

- Added `FrameStats::public_graph_execution` and made debug report/capture mirror the same explicit public graph execution data.
- Existing regression now verifies frame stats, debug report, and capture can all resolve the exported public buffer handle by label.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_ -- --nocapture` passed: 3 tests. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture` passed: 5 tests.

Remaining audit gap: this completes public observability for explicit graph exports, but does not yet make backend-wgpu surface/standard-frame graph outputs durable public resources.

## 2026-05-19 audit update: capture resource dump graph export counts

- `FrameCaptureResourceDump` now includes structured counts for explicit public graph exports and splits promoted exports from imported-resource exports.
- Added regression coverage for a promoted transient texture plus exported imported buffer, confirming resource dump counters and public buffer writeback.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_capture_resource_dump_counts_public_graph_exports -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer resource_dump -- --nocapture` passed: 2 tests. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture` passed: 5 tests.

Remaining audit gap: capture dump summarization for explicit public graph exports is implemented, but backend-wgpu surface/standard-frame graph output production is still not durable public frame output.

## 2026-05-19 audit update: graph export handle lifetime cleanup

- Destroying a texture or buffer handle referenced by the latest explicit public graph execution now clears the cached execution result so public graph export observability cannot expose stale handles.
- Added regression coverage for clearing on current exported buffer destruction and preserving a newer graph execution when destroying an older exported texture handle.
- While running the broader destroy filter, `frame_stats_report_resident_memory_and_delayed_destroy_count` exposed an outdated frame-latency assertion. The test now matches the implemented submitted-frame reclaim behavior: `SubmissionBoundary`, no delayed destroy count, and one reclaimed resource this frame.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer destroying_public_graph_export_handles_clears_last_graph_execution -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_stats_report_resident_memory_and_delayed_destroy_count -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer destroy -- --nocapture` passed: 13 tests. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture` passed: 5 tests.

Remaining audit gap: explicit public graph export handle lifetime is safer, but backend-wgpu surface/standard-frame graph output production and full backend-native pipeline cache coverage remain incomplete.

## 2026-05-19 audit update: non-surface public frame outputs

- Added durable public frame output reporting for headless and public texture-backed frame targets through `FrameStats`, `FrameDebugReport`, and `FrameCapture`.
- Headless targets now materialize a public texture output; texture/texture-view/external render targets expose their existing public color texture handle with source metadata.
- Surface targets intentionally remain `None` for public frame outputs so the renderer does not misrepresent swapchain output as a durable public resource.
- Updated resource-dump coverage because generated headless public frame output textures are now counted as ready textures.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_are_durable_public_textures -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer resource_dump -- --nocapture` passed: 2 tests. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_ -- --nocapture` passed: 3 tests. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed: 1 test.

Remaining audit gap: backend-wgpu surface/standard-frame graph output is still not produced as durable public frame output. Complete renderer goal remains open.

## 2026-05-19 audit update: headless public output clear bytes

- Headless durable public frame output textures now reflect `ClearOptions::ColorDepth` in their byte contents instead of being unconditional zero-filled resources.
- The regression checks a non-black clear color and verifies the generated public texture's first texel matches the encoded clear value.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_are_durable_public_textures -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer resource_dump -- --nocapture` passed: 2 tests. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed: 1 test.

Remaining audit gap: headless clear-output bytes do not complete backend-wgpu surface/swapchain public output export or full shaded frame readback.

## 2026-05-19 audit update: public texture-target frame writeback

- Direct public texture frame outputs now write camera clear color into the existing public texture bytes, so `FramePublicOutputSource::ExistingTargetTexture` is no longer handle-only for simple single-sample targets.
- External render target color textures share the same clear writeback implementation. TextureView subresource writeback remains intentionally open.
- Added regression coverage for a direct texture target with a non-black clear color and verified the original public texture's first texel changed to the encoded clear value.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer texture_frame_outputs_write_clear_color_to_existing_public_texture -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture` passed: 3 tests. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer resource_dump -- --nocapture` passed: 2 tests.

Remaining audit gap: TextureView subresource writeback, full shaded frame readback, and complete backend-wgpu surface/swapchain durable public output remain incomplete.

## 2026-05-19 audit note: public frame output writeback

The latest renderer slice closes part of the frame-output observability gap. Non-surface frame outputs are now represented as public renderer resources instead of being visible only through transient frame stats. Headless output creates a durable texture, while direct texture and external render-target outputs reuse the caller-owned public color texture and write the camera clear color into it.

Residual risk remains: texture-view outputs now write clear-color bytes for selected single-mip, 2D-compatible mip/layer ranges, and backend-wgpu surface readback is covered by a later partial slice. Multi-mip view output gaps from this slice remain, and the current non-surface readback content is clear-color data rather than the result of a full shaded render pipeline. This means the renderer layer is more observable and composable, but the complete full-frame output requirement is not fully satisfied yet.

Validation performed:

- `cargo test -p engine_renderer texture_frame_outputs_write_clear_color_to_existing_public_texture -- --nocapture` passed.
- `cargo test -p engine_renderer frame_outputs -- --nocapture` passed.
- `cargo test -p engine_renderer resource_dump -- --nocapture` passed.


## 2026-05-19 audit note: TextureView public frame output writeback

The TextureView frame-output gap has been reduced. `RenderTarget::TextureView` now preserves the existing public owner texture handle, reports the base-mip output extent, and writes camera clear-color bytes into the selected mip/layer range for 2D-compatible single-mip views.

This does not close the full frame-output contract yet. The remaining gap is shaded-frame content, multi-mip view output, and complete surface/swapchain public output.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer texture_view_frame_outputs_write_clear_color_to_target_subresource -- --nocapture` passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture` passed.

## 2026-05-19 audit note: public frame output subresource metadata

The public frame output observability surface now includes mip/layer metadata. This closes a gap introduced by TextureView writeback: callers can now distinguish whether an output handle represents the whole base texture target or a specific TextureView subresource.

Residual risk remains for full completion: the content is still clear-color writeback rather than shaded-frame readback, multi-mip render-target views are not supported in this slice, and complete surface/swapchain output still depends on the later backend-wgpu readback path plus remaining async/observability/export integration.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer texture_view_frame_outputs_write_clear_color_to_target_subresource -- --nocapture` passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture` passed.

## 2026-05-19 audit note: public frame output scene/material preview

The frame-output content gap is reduced: public frame output bytes now change based on real scene/material state instead of always mirroring camera clear color. The implemented path observes visibility, layer filtering, LOD resource selection, standard material base color, base-color texture average, and emissive values.

This must not be counted as complete shaded frame readback. It is a deterministic renderer-layer preview path that improves public API observability while full per-pixel rasterization, lighting, shadows, post-process readback, and surface/swapchain durable output remain open.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_use_base_color_texture_average -- --nocapture` passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture` passed.

## 2026-05-19 audit note: public frame output content provenance

Public frame outputs now expose whether their bytes are clear-only or derived from visible scene/material state. The new content provenance and contribution counts reduce ambiguity in stats/debug/capture: a caller can distinguish an empty clear output from a material-preview output and see how many visible geometry/material contributors were observed.

Residual risk remains: this is provenance for the preview path, not a replacement for full rasterized shaded-frame readback or durable surface/swapchain output.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_use_base_color_texture_average -- --nocapture` passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture` passed.

## 2026-05-19 audit note: public frame output lighting/environment preview

The public frame output preview now reacts to light, environment, and manual exposure state instead of only material state. The output payload reports light and environment contribution counts, which gives stats/debug/capture consumers a public signal that the durable output texture reflects more of the standard 3D scene context.

Residual risk remains high for final completion: this is still a preview path. It does not implement full rasterized shaded-frame readback, shadow resolve, post-process readback, or surface/swapchain durable output.

Validation performed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_include_light_environment_and_exposure_preview -- --nocapture` passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture` passed.

## Audit note - public frame output post-process preview (2026-05-19 18:00:29 +08:00)

Finding: frame output observability now includes render-path/post-process intent via deterministic preview effects and post_process_samples.

Evidence:
- New targeted coverage: headless_frame_outputs_include_post_process_preview.
- Regression coverage: rame_outputs test filter passed 7 tests.

Residual risk:
- The implementation does not yet prove that the real GPU post-process pipeline writes identical bytes to public outputs. This remains a renderer-layer completion gap and should stay tracked until backend execution/readback tests exist.

## Audit note - public graph mipped D2 base-mip import/export (2026-05-19 18:05:59 +08:00)

Finding: the explicit public graph execution path now handles imported D2 textures with mip_levels > 1 for base-mip upload/writeback instead of rejecting them outright.

Evidence:
- Added targeted test xecute_graph_to_resources_uploads_and_writes_back_mipped_2d_base_imports.
- Regression filter xecute_graph_to_resources_ passed 6 tests, including unsupported array/MSAA rejection.
- Regression test last_graph_execution_tracks_public_export_handles_and_clears_on_failure passed after switching its failure case to an actually unsupported array texture.

Residual risk:
- The RHI texture abstraction still exposes only one 2D region without mip/layer selectors. This change does not complete true multi-mip, array, cube, 3D, MSAA, or backend-wgpu surface graph export support.

## Audit note - public graph generated mip base upload (2026-05-19 18:08:01 +08:00)

Finding: generated D2 mip-chain textures are now usable as explicit public graph imports for base-mip execution. The import path extracts only the base mip, and writeback invalidates generated mip status after graph mutation.

Evidence:
- Added targeted test xecute_graph_to_resources_uploads_generated_mip_base_imports.
- xecute_graph_to_resources_ regression filter passed 7 tests.
- last_graph_execution_tracks_public_export_handles_and_clears_on_failure passed, preserving stale-result clearing on unsupported imports.

Residual risk:
- The current RHI region model still lacks mip/layer selectors, so this is not complete multi-mip graph execution. Remaining renderer goal items include true subresource graph IO and backend-wgpu standard-frame export integration.

## Audit note - headless/stub surface public frame outputs (2026-05-19 18:11:15 +08:00)

Finding: surface-target frame output observability is no longer entirely absent in headless/stub execution. Main-surface and valid surface-handle targets now expose durable public texture outputs when no backend runtime owns the swapchain.

Evidence:
- Added targeted test headless_surface_frame_outputs_are_durable_public_textures.
- rame_outputs regression filter passed 8 tests.

Residual risk:
- The backend-wgpu path intentionally remains backend-owned and does not expose a public readback texture yet. Full renderer completion still requires real swapchain/surface output export or readback integration.

## Audit note - public frame output multi-mip texture-view preview (2026-05-19 18:15:25 +08:00)

Finding: texture-view public frame outputs no longer reject valid multi-mip view ranges. The output payload now represents the selected mip range with deterministic packed preview bytes and metadata.

Evidence:
- Added targeted test 	exture_view_frame_outputs_write_clear_color_to_multi_mip_view.
- rame_outputs regression filter passed 9 tests.
- 	exture_view_render_targets_validate_subresource_ranges passed after accepting an in-range multi-mip view and still rejecting out-of-range views.

Residual risk:
- The stored layout model still cannot describe multiple mip/layer subresources for later RHI upload. This is an observable public-output preview improvement, not completion of true subresource graph/backend IO.

## Audit note - public frame output subresource byte layout (2026-05-19 18:18:02 +08:00)

Finding: public frame output payloads now carry explicit subresource byte-layout metadata, so multi-mip packed preview bytes are observable without caller-side inference.

Evidence:
- 	exture_view_frame_outputs_write_clear_color_to_target_subresource verifies the single-mip layer-range layout entry.
- 	exture_view_frame_outputs_write_clear_color_to_multi_mip_view verifies two packed mip layout entries and byte offsets.
- rame_outputs regression filter passed 9 tests.

Residual risk:
- Layout metadata does not imply the backend can render/read/write arbitrary mip/layer subresources. It is an observability improvement for the current public-output preview path.

## Audit note - public graph texture export descriptor metadata (2026-05-19 18:20:16 +08:00)

Finding: explicit public graph texture exports now carry descriptor snapshots directly in RendererGraphTextureExport, reducing the gap between graph export handles and tooling-readable resource metadata.

Evidence:
- xecute_graph_to_resources_promotes_exported_transients_to_public_handles now verifies promoted export metadata.
- xecute_graph_to_resources_uploads_and_writes_back_imported_exports now verifies imported export metadata.
- xecute_graph_to_resources_ passed 7 tests, and debug/capture plus last-graph regression tests passed.

Residual risk:
- The metadata is a snapshot of public graph execution output, not proof that backend-wgpu standard-frame or surface graph exports are durable public resources. Those remain renderer-goal gaps.

## Audit note - public graph buffer export descriptor metadata (2026-05-19 18:22:37 +08:00)

Finding: explicit public graph buffer exports now carry size and usage snapshots directly in RendererGraphBufferExport, matching the previously added texture descriptor export metadata.

Evidence:
- execute_graph_to_resources_promotes_exported_transients_to_public_handles now verifies promoted buffer export size/usage metadata.
- execute_graph_to_resources_uploads_and_writes_back_imported_exports now verifies imported buffer export size/usage metadata.
- execute_graph_to_resources_, debug/capture graph export, and last-graph regression tests all passed.

Residual risk:
- This is explicit graph export observability. It does not implement backend-wgpu standard-frame/surface graph exports as durable public resources.

## Audit note - public graph texture export represented subresource layout (2026-05-19 18:25:44 +08:00)

Finding: explicit public graph texture exports now expose represented subresource byte layout, preventing tools from confusing descriptor mip count with bytes actually exported by the current RHI path.

Evidence:
- execute_graph_to_resources_promotes_exported_transients_to_public_handles verifies promoted export subresource layout.
- execute_graph_to_resources_uploads_and_writes_back_imported_exports verifies imported export subresource layout.
- execute_graph_to_resources_uploads_and_writes_back_mipped_2d_base_imports verifies mip_levels remains 2 while subresources reports only mip 0.
- execute_graph_to_resources_, debug/capture graph export, and last-graph regression tests all passed.

Residual risk:
- Subresource layout metadata is not backend subresource execution. Full renderer completion still requires real RHI/backend support for mip/layer addressing and standard-frame/surface graph exports.

## Audit note - public graph buffer export represented byte range (2026-05-19 18:27:59 +08:00)

Finding: explicit public graph buffer exports now expose represented byte range metadata, making full-buffer export semantics direct instead of inferred from size alone.

Evidence:
- execute_graph_to_resources_promotes_exported_transients_to_public_handles verifies promoted buffer byte range metadata.
- execute_graph_to_resources_uploads_and_writes_back_imported_exports verifies imported buffer byte range metadata.
- execute_graph_to_resources_, debug/capture graph export, and last-graph regression tests all passed.

Residual risk:
- This is explicit graph buffer export observability. It does not implement backend-wgpu standard-frame/surface graph exports as durable public resources.

## Audit note - public graph export source provenance (2026-05-19 18:30:31 +08:00)

Finding: explicit public graph exports now report whether each exported resource came from promoted graph transient data or from an imported public resource writeback.

Evidence:
- execute_graph_to_resources_promotes_exported_transients_to_public_handles verifies PromotedTransient for promoted texture and buffer exports.
- execute_graph_to_resources_uploads_and_writes_back_imported_exports verifies ImportedPublic for imported texture and buffer exports.
- execute_graph_to_resources_uploads_and_writes_back_mipped_2d_base_imports verifies ImportedPublic for mipped D2 base-mip writeback.
- execute_graph_to_resources_, debug/capture graph export, and last-graph regression tests all passed.

Residual risk:
- Provenance metadata does not implement backend-wgpu standard-frame/surface graph exports as durable public resources. That remains a renderer-goal gap.

## Audit note - public graph texture export subresource coverage flags (2026-05-19 18:33:14 +08:00)

Finding: explicit public graph texture exports now summarize whether represented subresources fully cover the texture descriptor. This directly exposes the currently partial mipped-D2 base-mip path.

Evidence:
- execute_graph_to_resources_promotes_exported_transients_to_public_handles verifies full coverage for promoted single-mip D2 exports.
- execute_graph_to_resources_uploads_and_writes_back_imported_exports verifies full coverage for imported single-mip D2 exports.
- execute_graph_to_resources_uploads_and_writes_back_mipped_2d_base_imports verifies partial mip/subresource coverage for imported mipped D2 base-mip writeback.
- execute_graph_to_resources_, debug/capture graph export, and last-graph regression tests all passed.

Residual risk:
- Coverage flags are truthful observability, not implementation of missing subresource execution. Full renderer completion still requires real backend/RHI support for those paths.

## Audit note - public graph imported texture subregion upload (2026-05-19 18:35:17 +08:00)

Finding: explicit public graph imports now have tested coverage for partial TextureUpdate layout upload. The graph sees the updated texel at the declared offset and untouched texels remain zero until the pass writes full texture data.

Evidence:
- Added targeted test execute_graph_to_resources_uploads_imported_texture_subregions.
- execute_graph_to_resources_ regression filter passed 8 tests.

Residual risk:
- The represented export remains full base-mip readback. True public graph subregion export and full mip/layer backend subresource IO remain incomplete renderer-goal items.

## Audit note - public graph imported buffer subrange update coverage (2026-05-19 18:37:57 +08:00)

Finding: explicit public graph imports now have tested coverage for public buffer subrange updates. A non-zero offset BufferUpdate is visible to graph passes before graph execution, and graph export writes back full buffer bytes.

Evidence:
- Added targeted test execute_graph_to_resources_uploads_imported_buffer_subrange_updates.
- execute_graph_to_resources_ regression filter passed 9 tests.

Residual risk:
- Buffer import currently uploads the full merged public payload, not a minimal dirty range. Full renderer completion still requires backend-native scheduling/resource integration beyond this explicit graph path.

## Audit note - public graph buffer export byte coverage flag (2026-05-19 18:39:59 +08:00)

Finding: explicit public graph buffer exports now directly state whether the represented byte range covers the full buffer.

Evidence:
- execute_graph_to_resources_promotes_exported_transients_to_public_handles verifies full coverage for promoted buffer exports.
- execute_graph_to_resources_uploads_and_writes_back_imported_exports verifies full coverage for imported public buffer exports.
- execute_graph_to_resources_uploads_imported_buffer_subrange_updates verifies a subrange-updated public buffer still exports a full-buffer represented range.
- execute_graph_to_resources_, debug/capture graph export, and last-graph regression tests all passed.

Residual risk:
- Coverage metadata is truthful observability for current explicit graph semantics, not backend-native partial export support.

## Audit note - public graph export aggregate coverage helpers (2026-05-19 18:42:11 +08:00)

Finding: explicit public graph export coverage is now queryable at the aggregate execution level, not only per texture/buffer export.

Evidence:
- execute_graph_to_resources_promotes_exported_transients_to_public_handles verifies all-complete aggregate export coverage.
- execute_graph_to_resources_uploads_and_writes_back_imported_exports and subregion/subrange tests verify complete aggregate status for full represented exports.
- execute_graph_to_resources_uploads_and_writes_back_mipped_2d_base_imports verifies incomplete aggregate coverage for descriptor-mipped/base-mip-only texture export.
- execute_graph_to_resources_, debug/capture graph export, and last-graph regression tests all passed.

Residual risk:
- Aggregate helpers expose partial coverage; they do not close the underlying missing backend/RHI subresource implementation.

## Audit note - public frame output aggregate subresource helpers (2026-05-19 18:45:00 +08:00)

Finding: public frame output payload layout is now queryable at both per-output and frame aggregate levels, so tools can detect packed multi-subresource outputs directly.

Evidence:
- texture_view_frame_outputs_write_clear_color_to_target_subresource verifies single-subresource helper behavior and frame aggregate byte totals.
- texture_view_frame_outputs_write_clear_color_to_multi_mip_view verifies packed-subresource helper behavior and frame aggregate packed count/byte totals.
- frame_outputs regression filter passed 9 tests.

Residual risk:
- Helper methods expose layout truth for current preview/readback payloads; they do not close backend-wgpu surface readback or true RHI subresource execution gaps.

## Audit note - public graph imported D1 texture base-mip execution (2026-05-19 18:46:48 +08:00)

Finding: explicit public graph imports now support D1 texture base-mip execution and writeback, reducing the texture-shape gap beyond the existing D2 path.

Evidence:
- Added targeted test execute_graph_to_resources_uploads_and_writes_back_imported_d1_textures.
- execute_graph_to_resources_ regression filter passed 10 tests.

Residual risk:
- D1 support is implemented by using the current height-1 RHI-compatible path. It does not implement array/cube/3D/MSAA texture execution or true mip/layer addressing.

## Audit note - public graph imported D2Array flattened base-mip execution (2026-05-19 18:50:42 +08:00)

Finding: explicit public graph imports now support D2Array base-mip payloads through a flattened layer-stack RHI compatibility path, reducing the array-texture shape gap.

Evidence:
- Added targeted test execute_graph_to_resources_uploads_and_writes_back_imported_d2_array_textures.
- execute_graph_to_resources_rejects_unsupported_imported_texture_shapes now keeps Cube and MSAA rejection coverage.
- execute_graph_to_resources_ regression filter passed 11 tests.

Residual risk:
- The implementation is not true layer-aware RHI IO. It exposes array layers as a flattened 2D byte layout for explicit graph execution only; full renderer completion still requires native mip/layer addressing and backend-wgpu standard-frame/surface graph export integration.

## Audit note - public graph imported Cube/CubeArray flattened base-mip execution (2026-05-19 18:52:35 +08:00)

Finding: explicit public graph imports now support Cube base-mip payloads through a flattened face-stack RHI compatibility path, reducing the cube-texture shape gap.

Evidence:
- Added targeted test execute_graph_to_resources_uploads_and_writes_back_imported_cube_textures.
- execute_graph_to_resources_rejects_unsupported_imported_texture_shapes now keeps D3 and MSAA rejection coverage.
- execute_graph_to_resources_ regression filter passed 12 tests.

Residual risk:
- The implementation is not true cube-aware RHI IO. It exposes cube faces as a flattened 2D byte layout for explicit graph execution only; full renderer completion still requires native mip/layer/cube addressing and backend-wgpu standard-frame/surface graph export integration.

## Audit note - public graph imported D3 flattened base-mip execution (2026-05-19 18:54:23 +08:00)

Finding: explicit public graph imports now support D3 base-mip payloads through a flattened depth-stack RHI compatibility path, reducing the volume-texture shape gap.

Evidence:
- Added targeted test execute_graph_to_resources_uploads_and_writes_back_imported_d3_textures.
- execute_graph_to_resources_rejects_unsupported_imported_texture_shapes now keeps MSAA rejection coverage.
- execute_graph_to_resources_ regression filter passed 13 tests.

Residual risk:
- The implementation is not true volume-aware RHI IO. It exposes depth slices as a flattened 2D byte layout for explicit graph execution only; full renderer completion still requires native mip/layer/depth addressing and backend-wgpu standard-frame/surface graph export integration.

## Audit note - public graph imported Depth32Float D2Array flattened execution (2026-05-19 18:55:52 +08:00)

Finding: explicit public graph imports now have targeted Depth32Float D2Array coverage, proving flattened layer-stack support applies to depth values as well as color bytes.

Evidence:
- Added targeted test execute_graph_to_resources_uploads_and_writes_back_depth_array_imports.
- execute_graph_to_resources_ regression filter passed 14 tests.

Residual risk:
- The implementation is still flattened compatibility, not native depth-array RHI IO. Full renderer completion still requires true layer/depth addressing and backend-wgpu standard-frame/surface graph export integration.

## Audit note - public graph imported non-base mip represented execution (2026-05-19 18:59:48 +08:00)

Finding: explicit public graph imports can now execute a complete non-base mip payload rather than rejecting all mip_level > 0 texture layouts.

Evidence:
- Added targeted test execute_graph_to_resources_uploads_and_writes_back_imported_non_base_mips.
- execute_graph_to_resources_ regression filter passed 15 tests.

Residual risk:
- The implementation is represented-single-mip execution, not full multi-mip texture graph IO. Full renderer completion still requires native mip/layer/depth addressing and backend-wgpu standard-frame/surface graph export integration.

## Audit note - public graph imported non-base mip subregion execution (2026-05-19 19:01:56 +08:00)

Finding: explicit public graph imports can now execute an x/y subregion update inside a non-base mip while preserving truthful partial coverage metadata.

Evidence:
- Added targeted test execute_graph_to_resources_uploads_non_base_mip_subregions.
- execute_graph_to_resources_ regression filter passed 16 tests.

Residual risk:
- The implementation is represented-single-mip execution. Full renderer completion still requires simultaneous multi-mip and native mip/layer/depth backend addressing.

## Audit note - public graph texture import support query (2026-05-19 19:04:18 +08:00)

Finding: explicit public graph texture import capability is now queryable before execution, including truthful MSAA unsupported reasons and flattened compatibility status.

Evidence:
- Added targeted test execute_graph_to_resources_texture_import_support_reports_msaa_and_flattened_shapes.
- execute_graph_to_resources_ regression filter passed 17 tests.

Residual risk:
- Support queries improve tooling and preflight diagnostics only. Missing MSAA and native subresource backend execution remain open renderer-goal items.

## Audit note - public graph imported layer/depth subregion upload (2026-05-19 19:06:39 +08:00)

Finding: explicit public graph imports can now upload data written to a non-zero array layer into the correct flattened RHI position before graph execution.

Evidence:
- Added targeted test execute_graph_to_resources_uploads_imported_array_layer_subregions.
- execute_graph_to_resources_ regression filter passed 18 tests.

Residual risk:
- The implementation is still flattened compatibility. Full renderer completion still requires native mip/layer/depth backend addressing and standard-frame/surface graph export integration.

## Audit note - public graph buffer import support query (2026-05-19 19:08:30 +08:00)

Finding: explicit public graph buffer import capability is now queryable before execution, including represented full-buffer byte range semantics.

Evidence:
- execute_graph_to_resources_texture_import_support_reports_msaa_and_flattened_shapes now also verifies graph_buffer_import_support for a subrange-updated public buffer.
- execute_graph_to_resources_ regression filter passed 18 tests.

Residual risk:
- The query exposes current semantics only. Missing minimal-range upload scheduling and backend-native graph export integration remain open renderer-goal items.

## Audit note - public graph aggregate import support query (2026-05-19 19:10:14 +08:00)

Finding: explicit public graph import capability can now be queried for an entire graph before execution, including mixed supported and unsupported imported resources.

Evidence:
- execute_graph_to_resources_texture_import_support_reports_msaa_and_flattened_shapes now verifies graph_import_support on a mixed graph with MSAA texture, flattened D2Array texture, and public buffer imports.
- execute_graph_to_resources_ regression filter passed 18 tests.

Residual risk:
- Preflight support does not close missing execution features. native MSAA texture/sample-level graph execution and native backend subresource addressing remain open renderer-goal items.

## Audit note - public graph import preflight execution gate (2026-05-19 19:11:40 +08:00)

Finding: explicit public graph execution now consumes the same aggregate import support data exposed to tooling, so unsupported imports are rejected before RHI import creation with a public reason.

Evidence:
- execute_graph_to_resources_rejects_unsupported_imported_texture_shapes now verifies MSAA failure includes public graph import preflight and multisampled texture reason.
- execute_graph_to_resources_ regression filter passed 18 tests.

Residual risk:
- Preflight validation does not implement unsupported execution features; it only makes failure earlier and more observable.

## Audit note - public graph texture import represented layout support metadata (2026-05-19 19:13:32 +08:00)

Finding: explicit public graph texture import support now reports the exact represented layout and coverage flags that execution will use.

Evidence:
- execute_graph_to_resources_texture_import_support_reports_msaa_and_flattened_shapes verifies represented extent/bytes for MSAA preflight, flattened D2Array import, and non-base mip import.
- execute_graph_to_resources_ regression filter passed 18 tests.

Residual risk:
- The metadata is preflight observability. Missing MSAA and native subresource execution remain open renderer-goal items.

## Audit note - generated mip-chain import support query coverage (2026-05-19)

Finding: explicit public graph texture import support now reports generated mip-chain textures truthfully: the public descriptor owns multiple mips, but current graph import execution represents only the base mip.

Evidence:
- execute_graph_to_resources_uploads_generated_mip_base_imports now verifies graph_texture_import_support for a generated three-mip texture.
- The test verifies represented_mip = 0, represented base extent, represented base-mip byte length, complete layer coverage, incomplete mip coverage, and incomplete subresource coverage.
- execute_graph_to_resources_ regression filter passed 18 tests.

Residual risk:
- This closes support-query truthfulness for generated mip chains, not full multi-mip graph execution. Full renderer completion still requires simultaneous multi-mip IO, native backend subresource addressing, native MSAA texture/sample-level graph execution, and backend-wgpu standard-frame/surface export integration.

## Audit note - generated mip-chain graph writeback regeneration (2026-05-19)

Finding: explicit public graph writeback for generated mip-chain textures now regenerates and preserves the full retained mip payload after the graph modifies the represented base mip.

Evidence:
- execute_graph_to_resources_uploads_generated_mip_base_imports now verifies that graph execution reads the generated texture's base mip, writes updated base bytes, and the renderer stores the full regenerated 3-mip byte chain afterward.
- The same test verifies RendererGraphTextureExport reports three mip subresources with byte offsets and complete mip/layer/subresource coverage.
- execute_graph_to_resources_ regression filter passed 18 tests.

Residual risk:
- The graph execution path still operates on the represented base mip and regenerates the retained chain after writeback. This does not provide native multi-mip graph IO or backend subresource addressing.

## Audit note - layered generated mip-chain graph writeback coverage (2026-05-19)

Finding: explicit public graph writeback for generated mip chains now has layered D2Array coverage, not only single-layer D2 coverage.

Evidence:
- Added execute_graph_to_resources_regenerates_layered_generated_mip_exports.
- The test verifies flattened base-layer upload into graph execution, regenerated retained mip bytes for both layers, three exported mip subresource records, complete coverage flags, and TextureInfo::mips_generated remaining true.
- execute_graph_to_resources_ regression filter passed 19 tests.

Residual risk:
- This remains a represented base-mip execution plus retained-chain regeneration model. It does not close true native multi-mip or layer-addressed backend graph IO.

## Audit note - volume generated mip-chain graph writeback coverage (2026-05-19)

Finding: explicit public graph writeback for generated mip chains now has D3 volume coverage, and export coverage metadata accounts for shrinking volume depth per mip.

Evidence:
- Added execute_graph_to_resources_regenerates_volume_generated_mip_exports.
- The test verifies flattened base-volume upload into graph execution, regenerated retained volume mip bytes, three exported mip subresource records with depth counts 4/2/1, complete coverage flags, and TextureInfo::mips_generated remaining true.
- execute_graph_to_resources_ regression filter passed 20 tests.

Residual risk:
- This remains a represented base-mip execution plus retained-chain regeneration model. It does not close true native multi-mip or depth-addressed backend graph IO.

## Audit note - cube generated mip-chain graph writeback coverage (2026-05-19)

Finding: explicit public graph writeback for generated mip chains now has Cube texture coverage, not only D2/D2Array/D3 coverage.

Evidence:
- Added execute_graph_to_resources_regenerates_cube_generated_mip_exports.
- The test verifies flattened base-face upload into graph execution, regenerated retained cube mip bytes for all six faces, two exported mip subresource records, complete coverage flags, and TextureInfo::mips_generated remaining true.
- execute_graph_to_resources_ regression filter passed 21 tests.

Residual risk:
- This remains a flattened represented base-mip execution plus retained-chain regeneration model. It does not close native cube face/mip backend graph IO.

## Audit note - CubeArray generated mip-chain graph writeback coverage (2026-05-19)

Finding: explicit public graph writeback for generated mip chains now has CubeArray texture coverage, extending the generated-mip graph evidence beyond D2, D2Array, D3, and Cube.

Evidence:
- Added execute_graph_to_resources_regenerates_cube_array_generated_mip_exports.
- The test verifies flattened base-face/layer upload into graph execution, regenerated retained cube-array mip bytes for twelve faces/layers, two exported mip subresource records, complete coverage flags, and TextureInfo::mips_generated remaining true.
- execute_graph_to_resources_ regression filter passed 22 tests.

Residual risk:
- This remains a flattened represented base-mip execution plus retained-chain regeneration model. It does not close native cube-array face/layer/mip backend graph IO.

## Audit note - D1 generated mip-chain graph writeback coverage (2026-05-19)

Finding: explicit public graph writeback for generated mip chains now has D1 texture coverage, completing targeted generated-mip shape coverage for the current explicit graph compatibility model.

Evidence:
- Added execute_graph_to_resources_regenerates_d1_generated_mip_exports.
- The test verifies base-line upload into graph execution, regenerated retained D1 mip bytes, three exported mip subresource records, complete coverage flags, and TextureInfo::mips_generated remaining true.
- execute_graph_to_resources_ regression filter passed 23 tests.

Residual risk:
- The current model still executes one represented base mip and regenerates retained generated mips after writeback. It does not close native multi-mip backend graph IO.

## Audit note - generated mip-chain packed graph import read support (2026-05-19)

Finding: generated public texture imports now upload the complete generated mip chain into explicit graph execution as a packed RHI-compatible texture, allowing graph passes to read lower mips by packed y offset.

Evidence:
- execute_graph_to_resources_uploads_generated_mip_base_imports now verifies mip 0, mip 1, and mip 2 are all readable from the imported RHI texture before graph writes the base mip.
- graph_texture_import_support now reports complete coverage and packed represented height for the generated chain.
- execute_graph_to_resources_ regression filter passed 23 tests.

Residual risk:
- The packed representation is compatibility IO, not native mip/layer/depth backend addressing. Writeback still regenerates the retained chain from the base mip and does not retain graph-authored lower-mip edits.

## Audit note - generated lower-mip graph writeback retention (2026-05-19)

Finding: explicit public graph writeback no longer discards graph-authored lower mip edits for generated mip-chain imports.

Evidence:
- Added execute_graph_to_resources_preserves_authored_generated_lower_mip_writes.
- The test writes mip 1 through the packed graph representation, exports the texture, verifies public texture bytes preserve the authored lower mip and unchanged mip 2, verifies complete export coverage, and verifies TextureInfo::mips_generated becomes false.
- Base-only generated writeback tests continue to pass and keep TextureInfo::mips_generated true when lower mips are unchanged and regenerated from base.
- execute_graph_to_resources_ regression filter passed 24 tests.

Residual risk:
- The renderer still exposes packed compatibility coordinates rather than native graph subresource handles for each mip/layer/depth slice. Native backend subresource addressing remains open.

## Audit note - graph texture import support subresource layout metadata (2026-05-19)

Finding: explicit public graph texture import preflight now exposes represented subresource byte layout directly, matching export metadata.

Evidence:
- RendererGraphTextureImportSupport includes subresources.
- execute_graph_to_resources_uploads_generated_mip_base_imports verifies generated packed mip-chain import support reports three mip subresources with expected offsets and byte lengths.
- execute_graph_to_resources_ regression filter passed 24 tests.

Residual risk:
- The metadata describes the packed compatibility representation. Native backend subresource addressing remains open.

## Audit note - graph import support subresource aggregate helpers (2026-05-19)

Finding: graph import preflight can now summarize represented texture import layout at both per-import and aggregate graph levels.

Evidence:
- RendererGraphTextureImportSupport exposes subresource_byte_len and has_packed_subresources.
- RendererGraphImportSupport exposes texture_imports_with_packed_subresources, texture_imports_with_incomplete_subresource_coverage, texture_import_subresource_bytes, all_texture_imports_complete_subresource_coverage, and has_incomplete_import_coverage.
- execute_graph_to_resources_uploads_generated_mip_base_imports verifies per-import and aggregate helper values for a generated packed mip-chain import.
- execute_graph_to_resources_ regression filter passed 24 tests.

Residual risk:
- The helpers report current packed compatibility layout. Native backend subresource addressing remains open.

## Audit note - public texture info subresource layout metadata (2026-05-19)

Finding: public TextureInfo now exposes retained texture payload subresource layout directly, not only descriptor dimensions and mips_generated state.

Evidence:
- TextureInfo includes subresources plus complete mip/layer/subresource coverage flags.
- TextureInfo exposes subresource_byte_len and has_packed_subresources helpers.
- execute_graph_to_resources_preserves_authored_generated_lower_mip_writes verifies TextureInfo reports a three-mip packed retained payload after authored lower-mip graph writeback while TextureInfo::mips_generated is false.
- execute_graph_to_resources_ regression filter passed 24 tests.

Residual risk:
- TextureInfo describes retained public payload layout. It does not expose native backend resource subresource handles or backend-resident layout.

## Audit note - public buffer info byte coverage metadata (2026-05-19)

Finding: public BufferInfo now exposes retained buffer byte coverage directly, aligning buffer resource observability with graph buffer import/export metadata.

Evidence:
- BufferInfo includes byte_offset, byte_len, and complete_byte_coverage.
- BufferInfo exposes represented_byte_len and has_complete_byte_coverage helpers.
- execute_graph_to_resources_uploads_imported_buffer_subrange_updates verifies BufferInfo full-byte coverage after graph writeback.
- execute_graph_to_resources_ regression filter passed 24 tests.

Residual risk:
- BufferInfo describes retained public buffer bytes. It does not expose backend-resident dirty ranges or implement partial graph export ranges.

## Audit note - public buffer represented byte-range import support (2026-05-19)

Finding: explicit public graph buffer import now uses tracked represented byte ranges instead of always uploading/reporting full-buffer coverage before graph execution.

Evidence:
- StoredBuffer tracks represented_byte_offset and represented_byte_len.
- update_buffer merges updated ranges into the represented range.
- build_headless_rhi_imports_for_graph uploads only the represented byte range for imported public buffers.
- execute_graph_to_resources_uploads_imported_buffer_subrange_updates verifies pre-graph BufferInfo and graph_buffer_import_support report offset 2, length 3, incomplete coverage, while graph execution still observes the expected full zero-filled buffer plus updated subrange; after graph writeback BufferInfo returns to full coverage.
- execute_graph_to_resources_texture_import_support_reports_msaa_and_flattened_shapes verifies aggregate graph import support reports the incomplete represented buffer range.
- execute_graph_to_resources_ regression filter passed 24 tests.

Residual risk:
- The current model tracks one merged range. Multiple disjoint ranges and persistent backend dirty-range synchronization remain open.

## Audit note - public buffer disjoint represented byte-range import support (2026-05-19)

Finding: explicit public graph buffer import now preserves disjoint represented update ranges instead of expanding them into a single dirty span with false represented gaps.

Evidence:
- StoredBuffer tracks represented_byte_ranges in addition to bounding byte_offset/byte_len.
- BufferInfo and RendererGraphBufferImportSupport expose precise byte_ranges.
- build_headless_rhi_imports_for_graph uploads each represented range individually.
- execute_graph_to_resources_uploads_imported_buffer_subrange_updates now verifies two disjoint represented ranges, exact represented byte total, zero-filled gap behavior, and full coverage after graph writeback.
- execute_graph_to_resources_ regression filter passed 24 tests.

Residual risk:
- Dirty-range tracking is still retained-public-buffer/import focused. Persistent backend dirty synchronization and partial graph export ranges remain open.

## Audit note - graph buffer import represented-range aggregate helpers (2026-05-19)

Finding: graph import preflight can now summarize precise represented buffer ranges at both per-buffer and aggregate graph levels.

Evidence:
- RendererGraphBufferImportSupport exposes represented_byte_len and has_disjoint_byte_ranges.
- RendererGraphImportSupport exposes buffer_imports_with_incomplete_byte_coverage, buffer_imports_with_disjoint_byte_ranges, buffer_import_represented_bytes, and all_buffer_imports_complete_byte_coverage.
- execute_graph_to_resources_uploads_imported_buffer_subrange_updates verifies per-buffer and aggregate helper values for two disjoint represented buffer ranges.
- execute_graph_to_resources_ regression filter passed 24 tests.

Residual risk:
- The helpers report retained public-buffer import layout. Persistent backend dirty synchronization and partial graph export ranges remain open.

## Audit note - graph buffer export byte-range metadata alignment (2026-05-19)

Finding: explicit public graph buffer exports now expose byte_ranges and helper methods, matching BufferInfo and RendererGraphBufferImportSupport range metadata shape.

Evidence:
- RendererGraphBufferExport includes byte_ranges.
- RendererGraphBufferExport exposes represented_byte_len and has_disjoint_byte_ranges.
- execute_graph_to_resources_promotes_exported_transients_to_public_handles verifies promoted full-buffer export range metadata.
- execute_graph_to_resources_uploads_imported_buffer_subrange_updates verifies imported public buffer writeback full-buffer export range metadata.
- execute_graph_to_resources_ regression filter passed 24 tests.

Residual risk:
- Current explicit graph buffer exports still represent full buffers. Partial graph export ranges remain open.

## Audit note - partial graph buffer export ranges (2026-05-19)

Finding: explicit public graph buffer exports can now represent and write back a requested byte range instead of always reading back full buffers.

Evidence:
- RenderGraphBuilder::export_buffer_range records byte_offset/byte_len.
- RhiBufferExport carries the requested range.
- execute_graph_to_resources_uploads_imported_buffer_subrange_updates now exports only bytes 2..5 from an imported public buffer and verifies public retained bytes/range metadata update only that range.
- Added execute_graph_to_resources_promotes_partial_buffer_export_ranges for promoted transient partial buffer exports.
- execute_graph_to_resources_ regression filter passed 25 tests.

Residual risk:
- The API supports one export range per graph buffer. Multiple export ranges and backend standard-frame/surface export integration remain open.

## Audit note - imported buffer partial export range preflight (2026-05-19)

Finding: imported public buffer partial export ranges are now validated before graph execution, not discovered only during writeback.

Evidence:
- RenderGraphBuilder exposes exported_buffer_entries for facade preflight.
- execute_graph_to_resources validates imported public buffer export ranges against the public buffer size before RHI import creation.
- Added execute_graph_to_resources_rejects_imported_buffer_export_ranges_out_of_bounds.
- execute_graph_to_resources_ regression filter passed 26 tests.

Residual risk:
- Validation covers one range per exported graph buffer. Multiple export ranges and backend standard-frame/surface export integration remain open.

## Audit note - multiple graph buffer export ranges (2026-05-19)

Finding: explicit public graph buffer exports can now represent multiple disjoint byte ranges for one graph buffer.

Evidence:
- Added RenderGraphBuilder::export_buffer_ranges.
- RhiBufferExport now carries byte_ranges.
- execute_graph_to_resources_uploads_imported_buffer_subrange_updates uses two exported ranges from an imported public buffer and verifies only those ranges are written back.
- execute_graph_to_resources_promotes_partial_buffer_export_ranges uses two exported ranges from a transient graph buffer and verifies the promoted public buffer contains only those bytes.
- execute_graph_to_resources_ regression filter passed 26 tests.

Residual risk:
- Multi-range export is explicit graph/facade retained-resource behavior. Persistent backend dirty synchronization and backend standard-frame/surface export integration remain open.

## Audit note - buffer export range validation errors (2026-05-19)

Finding: explicit public graph buffer export ranges now have covered error paths for empty range lists and transient/imported out-of-bounds ranges.

Evidence:
- export_buffer_ranges(empty) now records an invalid empty range rather than falling back to full export.
- Added execute_graph_to_resources_rejects_invalid_transient_buffer_export_ranges.
- Existing imported range preflight test still covers public buffer out-of-bounds ranges.
- execute_graph_to_resources_ regression filter passed 27 tests.

Residual risk:
- Validation is explicit graph/facade scoped. Backend standard-frame/surface export integration remains open.

## Audit note - graph buffer export range aggregate helpers (2026-05-19)

Finding: explicit graph buffer export range metadata is now queryable at the aggregate export payload level.

Evidence:
- RendererGraphResourceExports exposes buffer_exports_with_disjoint_byte_ranges and buffer_export_represented_bytes.
- execute_graph_to_resources_uploads_imported_buffer_subrange_updates verifies aggregate values for an imported public buffer with two exported ranges.
- execute_graph_to_resources_promotes_partial_buffer_export_ranges verifies aggregate values for a promoted transient buffer with two exported ranges.
- execute_graph_to_resources_ regression filter passed 27 tests.

Residual risk:
- The helpers summarize explicit graph export metadata only. Backend standard-frame/surface export integration remains open.

## Audit note - buffer export range normalization (2026-05-19)

Finding: explicit graph buffer export range metadata is now canonicalized before validation/readback, preventing overlapping or adjacent range requests from being double-counted.

Evidence:
- export_buffer_ranges sorts ranges and merges overlapping/adjacent ranges.
- execute_graph_to_resources_uploads_imported_buffer_subrange_updates now passes unordered adjacent ranges and verifies normalized exported/retained metadata.
- execute_graph_to_resources_ regression filter passed 27 tests.

Residual risk:
- Normalization is scoped to explicit graph/facade export ranges. Backend standard-frame/surface export integration remains open.
## 2026-05-19 audit update: D1/D2/layered texture region export slice

Closed gap:

- Public RenderGraph texture exports now support explicit D1/D2 base-mip rectangular region export, D2 non-base mip rectangular region export, D2Array single-layer and multi-layer non-base mip region export, Cube/CubeArray single-face and aligned multi-layer non-base mip region export, D3 non-base mip aligned depth-slice region export, D1/D2 generated base/lower-mip rectangular region export, aligned whole-layer D2Array/D3/Cube/CubeArray base-mip region export, and single-layer/cross-layer partial-layer flattened D2Array/D3/Cube/CubeArray region export through `export_texture_region`.
- `Renderer::graph_texture_region_export_support` now exposes public preflight for texture region export support, including the subresource metadata and coverage flags that execution would report, including multi-subresource metadata for cross-layer partial-layer flattened regions.
- `Renderer::graph_region_export_support` now exposes graph-level batch preflight for imported public texture region exports, preserving deterministic export labels and aggregating supported/unsupported counts, boolean unsupported gates, supported/unsupported label lists, unsupported reasons, reason count/bool helpers, label+reason summaries, label coverage, and subresource bytes before execution.
- `Renderer::graph_import_support` now carries the same imported texture region export preflight results, aggregate helpers, boolean gate helpers, reason count/bool helpers, and label+reason summaries, aligning the existing graph-level support query with explicit region export gates.
- `RendererGraphTextureExport.region` now exposes the requested export rectangle directly on the public graph-to-resource result.
- `RendererGraphTextureExport::{subresource_byte_len, has_packed_subresources}` and `RendererGraphResourceExports` aggregate helpers now expose packed/multi-subresource graph export observability without requiring tools to manually inspect the subresource vector.
- `RendererGraphResourceExports` now exposes aggregate texture/buffer export, promoted-export, imported-export, region-export, total-count, label-count, and label-coverage query helpers so callers can detect export counts, promoted/imported counts and labels, and region exports without manually scanning every export.
- `RenderGraphStats` now carries exported texture region counts and labels, exposes `has_texture_region_exports()`, label-count/label-coverage helpers, sorted label helpers, and graph stat accumulation/merge preserves them for frame-level observability.
- `RenderGraphStats` now carries backend-origin exported texture region counts and labels through `backend_exported_texture_regions` and `backend_exported_texture_region_labels`, plus backend-specific label-count/coverage and sorted-label helpers, so backend-wgpu/native graph stats can retain provenance after facade/backend merge.
- `FrameStats`, `FrameProfile`, `FrameDebugReport`, and `FrameCapture` now include direct public graph exported texture/buffer counts and labels, closing flat observability for ordinary explicit graph exports as well as region exports.
- `FrameStats`, `FrameProfile`, `FrameDebugReport`, and `FrameCapture` now include direct promoted public graph texture/buffer counts and labels, closing flat observability for transient-promoted graph outputs outside capture resource dumps.
- `FrameStats`, `FrameProfile`, `FrameDebugReport`, and `FrameCapture` now include direct imported public graph texture/buffer counts and labels, closing flat observability for imported-resource graph exports outside capture resource dumps.
- `FrameCapture` now mirrors public graph export flat fields from `FrameStats`, including texture-region fields, instead of recalculating them from nested graph execution data.
- `FrameStats::{public_graph_export_count, public_graph_promoted_export_count, public_graph_imported_export_count, public_graph_export_label_count, public_graph_promoted_export_label_count, public_graph_imported_export_label_count, public_graph_texture_region_export_label_count, has_complete_public_graph_texture_region_export_label_coverage, has_complete_public_graph_export_label_coverage}` provide immediate-frame aggregate helper coverage for public graph export tooling.
- `FrameProfile`, `FrameDebugReport`, and `FrameCapture` expose matching public graph export aggregate helpers, keeping profile/debug/capture tooling aligned with `FrameStats`.
- `FrameStats` now includes direct `public_graph_texture_region_exports` and `public_graph_texture_region_export_labels` fields, closing immediate frame-output observability without requiring consumers to inspect nested graph execution data.
- `FrameProfile` now includes direct `public_graph_texture_region_exports` and `public_graph_texture_region_export_labels` fields, closing profiling payload observability for the same explicit graph region export data.
- `FrameCaptureResourceDump` now includes `public_graph_exported_texture_labels`, `public_graph_exported_buffer_labels`, `public_graph_promoted_texture_labels`, `public_graph_promoted_buffer_labels`, `public_graph_imported_texture_export_labels`, `public_graph_imported_buffer_export_labels`, `public_graph_texture_region_exports`, and `public_graph_texture_region_export_labels`, closing the capture/resource-dump observability path for explicit public graph export counts, labels, and promoted/imported classification.
- `FrameCaptureResourceDump::{public_graph_export_count, public_graph_promoted_export_count, public_graph_imported_export_count, public_graph_export_label_count, public_graph_promoted_export_label_count, public_graph_imported_export_label_count, public_graph_texture_region_export_label_count, has_complete_public_graph_texture_region_export_label_coverage, has_complete_public_graph_export_label_coverage}` provide public helper coverage so capture tooling can verify export totals and label/count consistency without manually comparing every field.
- `FrameDebugReport` now includes direct `public_graph_texture_region_exports` and `public_graph_texture_region_export_labels` fields, closing the editor/debug observability path for explicit public graph texture region exports without requiring consumers to inspect nested graph execution data.
- `FrameCapture` now includes the same direct `public_graph_texture_region_exports` and `public_graph_texture_region_export_labels` fields, closing capture payload observability without requiring capture consumers to inspect nested graph execution data.
- The graph export region is preserved through compiled resource exports and RHI export metadata.
- Transient graph texture promotion reads only the requested rectangle and promotes it to a public texture whose descriptor keeps the original full extent while metadata reports the represented partial subresource.
- Imported public D2 texture writeback reads only the requested rectangle and updates the public resource bytes/layout to the represented partial region.
- Imported public D2 non-base mip region writeback uses represented graph/RHI mip coordinates and maps the result to public mip-level offset/extent metadata.
- Imported public D2Array single-layer and aligned multi-layer non-base mip region writeback uses flattened layer coordinates and maps them back to public mip-level, base-layer, layer-count, offset, and extent metadata.
- Imported public Cube/CubeArray single-face and aligned multi-layer non-base mip region writeback uses flattened face coordinates and maps them back to public mip-level, face/base-layer, layer-count, offset, and extent metadata.
- Imported public D3 non-base mip region writeback uses flattened mip depth-slice coordinates and maps them back to public mip-level, base-layer/depth-slice, offset, and extent metadata.
- Imported public D1/D2 generated mip-chain region writeback reads only the selected base or lower mip-local region from the packed RHI chain, stores it as a partial public subresource layout, and clears generated-mip status.
- Imported public D1 texture writeback supports `y=0,height=1` range exports and rejects invalid D1 y/height regions before graph execution.
- Imported public D2Array/D3/Cube/CubeArray writeback supports flattened regions aligned to full layer/depth-slice height and maps them back to public `base_layer`, `layer_count`, `offset`, and extent metadata.
- Imported public D2Array/D3/Cube/CubeArray writeback supports partial flattened regions across one or more layers/depth-slices, maps flattened `y` back to public mip-local `offset.y`, and splits cross-layer partial exports into multiple public subresources when needed.
- Cross-layer partial flattened writeback retains enough multi-subresource public texture layout metadata for later public graph imports to upload the compact bytes back into the correct flattened RHI coordinates.
- Layered region preflight rejects out-of-bounds flattened regions before graph execution.
- Export coverage now includes 2D extent completeness, preventing partial rectangles from being misreported as complete subresource coverage.
- Texture export and texture info subresource metadata now includes `offset`, closing the previous observability gap where a partial rectangle exposed only width/height.
- Texture import support now reports offset-aware public subresource metadata instead of only the RHI-normalized upload extent; `represented_*` fields continue to expose the upload-compatible extent.
- Public imported texture preflight rejects invalid D1/D2 region exports before executing graph passes.

Validation:

```powershell
& 'C:\Users\JM\.cargo\bin\cargo.exe' test -p engine_renderer execute_graph_to_resources_ -- --nocapture
```

Result: `49 passed; 0 failed; 0 ignored; 273 filtered out`.

Additional targeted validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture` passed, 1 passed. The test now covers explicit public graph texture/buffer export flat counts and labels, promoted export flat counts and labels, imported export zero/empty flat fields, texture region export handles, compact region readback bytes, `FrameStats::public_graph_execution`, direct `FrameStats` region-export fields, direct `FrameProfile` region-export fields, `FrameCapture::public_graph_execution`, direct `FrameDebugReport` region-export fields, and direct `FrameCapture` region-export fields.

Additional imported-export validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_capture_resource_dump_counts_public_graph_exports -- --nocapture` passed, 1 passed. The test now covers non-zero imported texture and buffer exports through `FrameStats`, `FrameProfile`, `FrameDebugReport`, `FrameCapture`, `FrameCaptureResourceDump`, and public texture/buffer writeback.

Aggregate helper validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer public_graph_export -- --nocapture` passed, 3 passed. The filter covers public graph export handle cleanup, frame/debug/capture public graph export observability, resource dump public graph export counts, `RendererGraphResourceExports` aggregate helpers, and the aggregate helpers on `FrameStats`, `FrameProfile`, `FrameDebugReport`, and `FrameCapture` for promoted-only and imported texture/buffer paths.

Additional Cube/CubeArray non-base-mip multi-layer validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_writes_back_imported_cube_non_base_mip_multi_layer_texture_export_regions -- --nocapture` passed, 1 passed. The test covers Cube and CubeArray aligned multi-layer writeback from represented non-base mip coordinates, public layer-count metadata, incomplete mip/layer/subresource coverage flags, public texture bytes, and `TextureInfo` subresource observability.

Additional generated lower-mip region validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_writes_back_generated_lower_mip_texture_export_regions -- --nocapture` passed, 1 passed. The test covers D2 generated packed mip-chain region export from mip1, public `mip_level` and `offset` metadata, partial coverage flags, public texture bytes, and `TextureInfo` subresource observability.

Additional partial-layer validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer partial_layer -- --nocapture` passed, 2 passed, and `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer cross_layer_partial -- --nocapture` passed, 1 passed. These filters cover D2Array/D3/Cube/CubeArray single-layer partial flattened region writeback, cross-layer partial flattened region writeback split into multiple public subresources, execution-result packed-subresource helpers, re-import/upload of multi-subresource public texture bytes, and out-of-bounds rejection through the broader graph-to-resource filter.

Additional backend provenance validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_stats_preserves_import_export_resource_observability -- --nocapture` passed, 1 passed, and `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed, 1 passed. These tests cover aggregate and backend-specific texture region export count/label accumulation, sorted labels, label coverage helpers, and facade/backend graph-stat merge provenance.

Additional public preflight validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer region_export_support -- --nocapture` passed, 2 passed. The filter covers supported single-layer partial-layer region metadata, supported cross-layer multi-subresource metadata and packed-subresource helpers, unsupported out-of-bounds boundaries, graph-level imported texture region export labels, supported/unsupported aggregate counts and label lists, boolean unsupported gates, label coverage, aggregate subresource bytes, deterministic ordering, unsupported reasons, reason count/bool helpers, and label+reason summaries through public support queries, and `graph_import_support(&graph)` parity for imported texture region export gates.

Residual risks:

- This is a headless/RHI graph-to-resource closure, not a backend-wgpu frame/surface closure.
- Region export support is limited to D1/D2 base-mip, D2 non-base mip, D2Array single-layer and multi-layer non-base mip, Cube/CubeArray single-face and aligned multi-layer non-base mip, D3 non-base mip aligned depth-slice, D1/D2 generated base/lower-mip, aligned whole-layer D2Array/D3/Cube/CubeArray base-mip, and single-layer partial-layer flattened public texture semantics in this slice.
- Backend-wgpu native region export execution/readback behavior remains open.
- Full renderer goal remains open because standard 3D renderer, backend-wgpu execution, frame observability, full resource lifecycle, examples, and remaining matrix items are not complete.

## 2026-05-19 - Backend-wgpu surface public output readback audit

Closed gap:

- Backend-owned wgpu surface outputs no longer always disappear from `FrameStats.public_frame_outputs`.
- `graphics_wgpu::WgpuSurface` opts surface textures into `COPY_SRC` when supported, copies the completed surface image into a CPU readback buffer, and stores a `WgpuFrameReadback` snapshot.
- Backend-wgpu surface readback is now opt-in instead of being enabled automatically by `WgpuRendererRuntime::with_surface`; `Renderer::surface_frame_readback_supported()`, `Renderer::surface_frame_readback_enabled()`, `Renderer::set_surface_frame_readback_enabled()`, `Renderer::request_surface_frame_readback_next_frame()`, and `Renderer::cancel_surface_frame_readback_next_frame()` expose public control.
- `Renderer::surface_frame_readback_pending()`, `Renderer::surface_frame_readback_available()`, `Renderer::poll_surface_frame_readback()`, and `Renderer::materialize_surface_frame_readback(label)` expose public ready/poll/materialize behavior for completed backend surface readbacks outside the originating frame.
- `request_surface_frame_readback_next_frame()` temporarily enables surface readback for the next successfully finished frame and restores the previous state after `Frame::finish()`, reducing the risk that tooling leaves synchronous readback enabled across normal gameplay frames.
- When enabled, `WgpuSurface` records a pending readback after render submission instead of blocking inside `render_frame()`, exposes nonblocking `try_resolve_pending_frame_readback()`, and keeps an explicit blocking `resolve_pending_frame_readback()` for callers that choose to wait.
- `Renderer::public_frame_output_for_view` now materializes backend main-surface and backend surface-handle readback into durable public texture handles using `FramePublicOutputSource::BackendMainSurfaceReadback` or `FramePublicOutputSource::BackendSurfaceReadback`.
- `Renderer::public_frame_output_for_view` uses nonblocking try-resolve when it needs to materialize a backend surface public output; not-ready readbacks become `BackendSurfaceReadbackUnavailable` observability instead of a frame stall.
- The materialized public texture preserves width, height, format, row layout, bytes, usage flags, and frame-output subresource metadata.
- When backend surface output cannot be materialized, `FrameStats`, `FrameDebugReport`, and `FrameCapture` now expose `unsupported_public_frame_outputs` entries with explicit backend surface readback unsupported/disabled/unavailable reasons instead of silently omitting the output.
- The local window/surface smoke path uncovered and fixed a real wgpu validation issue: reflected material post-pass submissions were drawn in the post-process render pass while the sampled post-process pipeline and pass depth attachment state disagreed. `render_wgpu::MeshRenderer` now keeps the post-process pass plus color/sampled post-process pipelines depth-compatible when a surface depth attachment exists.
- `render_facade_window_usecase --require-surface-readback` now treats either an explicit `materialize_surface_frame_readback()` texture or a backend surface readback `FramePublicFrameOutput` as successful durable readback evidence.

Validation:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p graphics_wgpu surface_readback_layout_supports_public_color_formats -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_surface_readback_materializes_durable_public_frame_output -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer unsupported_public_frame_outputs_propagate_to_debug_report_and_capture -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer surface_frame_readback_api_requires_backend_surface_renderer -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe build -p render_facade_window_usecase` passed after the depth-compatible post-process pipeline/pass and example success-condition fixes.
- `.\target\debug\render_facade_window_usecase.exe --smoke-frames 8 --wait-for-gpu --print-stats --surface-readback --require-surface-readback` passed on the local visible window/surface path with exit 0 and reported `public_outputs=1`, `unsupported_public_outputs=0`, `surface_readback_frame_outputs=1`, `draws=9`, `visible=2`, and `gpu_time_ms=Some(1.61932)`.

Remaining audit gap:

- Surface render no longer blocks unconditionally on readback, backend surface public output materialization uses nonblocking try-resolve, and public ready/poll/materialize APIs now exist for completed readbacks outside the originating frame.
- Remaining production work is backend graph export integration, additional real-device/platform surface coverage beyond this local smoke, and native texture-region graph export/readback.
- Surface formats or platforms without `COPY_SRC` support still produce no durable public surface output, but the failure reason is now propagated through stats/debug/capture.
- This does not implement backend-wgpu RenderGraph exported surface resources, durable graph promotion, or native backend texture-region export/readback.
- Complete renderer goal remains open.

## 2026-05-20 - Backend-wgpu texture-region graph export/readback audit

Closed gap:

- Added direct backend-wgpu proof for transient public graph texture-region export/readback.
- `graph_execute_on_wgpu_exports_texture_region_with_readback` creates a 4x4 RGBA8 transient graph texture, writes deterministic data through a graph callback on `WgpuRhiDevice`, exports a 2x2 region, verifies the `RhiTextureExportRegion` metadata, and reads the same 2x2 region back through `WgpuRhiDevice::read_texture_rgba8`.
- `graph_execute_on_wgpu_exports_float_and_depth_regions_with_readback` extends the backend-wgpu proof to RGBA16F, RGBA32F, and Depth32Float region export/readback.
- Depth32Float writes now execute through `WgpuRhiDevice::write_texture_depth32f` using a depth-only render pass that writes `frag_depth` from a storage buffer, avoiding wgpu's forbidden `Queue::write_texture` depth-copy path while producing readable depth values.
- `wgpu_rhi_write_texture_depth32f_writes_readable_region` covers the direct backend-wgpu RHI write/readback path outside RenderGraph.

Validation:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_wgpu_exports_texture_region_with_readback -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_wgpu_exports_float_and_depth_regions_with_readback -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_rhi_write_texture_depth32f_writes_readable_region -- --nocapture` passed, 1 passed.

Remaining audit gap:

- This closes backend-wgpu transient D2 RGBA8/RGBA16F/RGBA32F/Depth32Float region-export proofs and true Depth32Float write/readback execution for the current D2 RHI path. It does not yet close backend-wgpu standard-frame/surface graph export/promotion, native multi-mip/layer/depth region addressing, or broader real-device/platform matrix coverage.
- Complete renderer goal remains open.

## 2026-05-20 - Backend-wgpu imported public texture writeback audit

Closed gap:

- Added direct backend-wgpu proof for explicit public graph imported texture upload and exported-import writeback.
- `execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_texture_shapes` creates a `Renderer::new` with `BackendPreference::Wgpu`, imports public textures into `RenderGraphBuilder`, verifies the backend-wgpu RHI sees the uploaded initial bytes before graph writes, writes updated bytes through the graph callback, exports the imported graph texture, and verifies the original public `TextureHandle` receives the updated bytes.
- Shape coverage is D1, D2, D2Array, D3, Cube, and CubeArray for RGBA8 through the current flattened-compatible base-mip representation.
- Format coverage includes D2 RGBA16F, D2 RGBA32F, D2 Depth32Float, and flattened D2Array Depth32Float on the backend-wgpu RHI path.
- `execute_graph_to_resources_wgpu_writes_back_imported_rgba8_texture_export_region`, `execute_graph_to_resources_wgpu_writes_back_imported_float_texture_export_regions`, and `execute_graph_to_resources_wgpu_writes_back_imported_depth_texture_export_region` cover imported public D2 RGBA8, D2 RGBA16F/RGBA32F, and D2 Depth32Float `export_texture_region` writeback on backend-wgpu, including partial public bytes/layout and incomplete subresource coverage metadata.
- `execute_graph_to_resources_wgpu_writes_back_imported_layered_rgba8_texture_export_regions` covers D2Array, D3, Cube, and CubeArray RGBA8 whole-layer/face flattened region writeback on backend-wgpu.
- `execute_graph_to_resources_wgpu_writes_back_imported_cross_layer_rgba8_texture_export_regions` covers D2Array, D3, Cube, and CubeArray RGBA8 cross-layer/cross-face flattened partial region writeback on backend-wgpu, including multi-subresource public layout metadata.
- `execute_graph_to_resources_wgpu_writes_back_imported_non_base_mip_rgba8_texture_export_region` covers represented non-base mip D2 RGBA8 region writeback on backend-wgpu, including mip-level metadata and incomplete mip/subresource coverage.
- `execute_graph_to_resources_wgpu_regenerates_generated_mip_imports` covers generated D2 RGBA8 mip-chain upload/read/writeback/regeneration on backend-wgpu; graph execution observes base/mip1/mip2 in the packed import representation, and writeback regenerates lower mips when only the base mip changes.
- `execute_graph_to_resources_wgpu_regenerates_generated_mip_import_shapes` covers generated D1, D2Array, D3, Cube, and CubeArray RGBA8 mip-chain packed import/writeback/regeneration on backend-wgpu.

Validation:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_texture_shapes -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_rgba8_texture_export_region -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_float_texture_export_regions -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_depth_texture_export_region -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_layered_rgba8_texture_export_regions -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_cross_layer_rgba8_texture_export_regions -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_non_base_mip_rgba8_texture_export_region -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_regenerates_generated_mip_imports -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_regenerates_generated_mip_import_shapes -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_ -- --nocapture` passed, 12 passed.

Remaining audit gap:

- This closes backend-wgpu imported public texture writeback for current single-mip and packed/generated-mip flattened-compatible color shapes plus D2 float/depth formats and flattened D2Array depth, including D2 RGBA8/RGBA16F/RGBA32F/Depth32Float partial region writeback, D2Array/D3/Cube/CubeArray RGBA8 whole-layer/face plus cross-layer/cross-face flattened region writeback, represented non-base mip D2 RGBA8 region writeback, and generated D1/D2/D2Array/D3/Cube/CubeArray RGBA8 mip-chain regeneration. It does not complete standard-frame/surface graph export/promotion, native simultaneous multi-mip/layer/depth addressing, native MSAA texture/sample-level graph execution, persistent backend-resident dirty synchronization, or broader platform coverage.
- Complete renderer goal remains open.

## 2026-05-20 - Backend-wgpu transient graph export promotion audit

Closed gap:

- Added direct backend-wgpu proof that explicit public graph transient exports can become durable public renderer resources.
- `execute_graph_to_resources_wgpu_promotes_exported_transients_to_public_handles` creates a `Renderer::new` with `BackendPreference::Wgpu`, writes transient graph D2 RGBA8/RGBA16F/RGBA32F/Depth32Float textures and a transient graph buffer through the backend-wgpu RHI path, exports them, and verifies `Renderer::execute_graph_to_resources` returns promoted public `TextureHandle` / `BufferHandle` resources with matching public bytes.
- The test verifies promoted/export source metadata, descriptor metadata, complete texture subresource coverage for each covered format, complete buffer byte coverage, and label lookup through `RendererGraphResourceExports`.
- `execute_graph_to_resources_wgpu_promotes_partial_transient_texture_and_buffer_exports` covers transient D2 RGBA8/RGBA16F/RGBA32F/Depth32Float texture-region promotion and transient buffer disjoint-range promotion on backend-wgpu, including incomplete coverage metadata and durable public bytes.

Validation:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_promotes_exported_transients_to_public_handles -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_promotes_partial_transient_texture_and_buffer_exports -- --nocapture` passed, 1 passed.

Remaining audit gap:

- This closes backend-wgpu explicit graph transient D2 RGBA8/RGBA16F/RGBA32F/Depth32Float texture and buffer promotion, plus partial transient D2 RGBA8/RGBA16F/RGBA32F/Depth32Float texture-region and disjoint buffer-range promotion. It does not complete backend-wgpu standard-frame/surface graph export/promotion, native multi-shape/multi-mip transient texture promotion, persistent backend-resident graph resource lifetime, or broader platform coverage.
- Complete renderer goal remains open.

## 2026-05-20 - Backend-wgpu imported public buffer writeback audit

Closed gap:

- Added direct backend-wgpu proof for explicit public graph imported buffer upload and exported-import writeback.
- `execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_buffer_export` creates a `Renderer::new` with `BackendPreference::Wgpu`, imports a public buffer into `RenderGraphBuilder`, verifies the backend-wgpu RHI sees the uploaded initial bytes before graph writes, writes updated bytes through the graph callback, exports the imported graph buffer, and verifies the original public `BufferHandle` receives the updated bytes.
- The test verifies `ImportedPublic` provenance, full-buffer byte range metadata, complete byte coverage, and label lookup through `RendererGraphResourceExports`.
- `execute_graph_to_resources_wgpu_writes_back_imported_buffer_export_ranges` covers partial/disjoint imported buffer export ranges on backend-wgpu, including incomplete byte coverage metadata and retained public buffer byte-range layout.
- `WgpuRhiDevice` now keeps backend buffers at a 4-byte-aligned physical size while preserving the public/RHI logical size. `write_buffer` and `read_buffer` avoid wgpu validation panics for renderer-valid unaligned byte ranges by using aligned physical readback/read-modify-write ranges and slicing caller-visible bytes, including partial ranges at the end of non-4-byte-sized buffers.

Validation:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_buffer_export -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_buffer_export_ranges -- --nocapture` passed, 1 passed, including a 7-byte public buffer with end-of-buffer partial export range writeback.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_ -- --nocapture` passed, 4 passed.

Remaining audit gap:

- This closes backend-wgpu imported public buffer writeback for full-buffer exports and partial/disjoint byte-range exports. It does not complete persistent backend-resident dirty-range synchronization, standard-frame/surface graph export/promotion, or broader platform coverage.
- Complete renderer goal remains open.

## 2026-05-20 - Standard-frame graph extension export promotion audit

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- Public `RenderGraphExtension` exports declared during standard frame rendering are no longer stats-only. The frame graph now runs through the RHI export path when exports are present and promotes exported transient textures/buffers into durable public handles.
- `last_graph_execution`, `FrameStats`, `FrameDebugReport`, `FrameCapture`, and resource-dump counters can now observe promoted frame graph extension exports.
- backend-wgpu runtime uses the wgpu RHI device for the standard-frame promotion path; headless renderer uses `HeadlessRhiDevice`.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_exports -- --nocapture` passed, 5 passed.

Still open:
- Readback-backed surface graph export promotion is implemented; direct swapchain image graph export remains a platform/wgpu capability-gated boundary. Surface public frame readback exists, but graph extension export promotion is still not a native surface export path.
- Native simultaneous mip/layer/depth graph resource addressing remains open.
- MSAA graph import/resolve remains open.
- Persistent backend-resident graph resource synchronization remains open.
- Full renderer-layer completion remains blocked by the remaining `Partial` and `Missing` matrix items.

Additional aggregation audit:
- The frame graph promotion path records the current renderer frame index and merges same-frame promoted export executions, so multi-view frame graph exports do not overwrite earlier promoted handles in the same frame.


Frame-index audit note:
- The frame graph export accumulator is keyed by the active frame index passed through `Frame::finish`, so overridden frame indices do not accidentally merge with unrelated renderer-default frames.

## 2026-05-20 - Resolved MSAA public graph import audit

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- Public multisampled D2 textures can enter explicit graph execution as resolved single-sample payloads.
- Full texture export/writeback preserves the original public texture handle and multisample metadata.
- Texture-region export/writeback works on the resolved representation and reports partial coverage.
- backend-wgpu uses the same resolved compatibility path through the wgpu RHI device.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer msaa -- --nocapture` passed, 6 passed.

Still open:
- Native sample-level MSAA graph textures and programmable/native resolve remain unimplemented because the current RHI graph texture descriptor still has no sample-count/subsample representation.
- Native multi-mip/layer/depth graph resource addressing remains open.
- Readback-backed surface graph export promotion is implemented; direct swapchain image graph export remains a platform/wgpu capability-gated boundary.
- Persistent backend-resident graph resource synchronization remains open.
- Full renderer-layer completion remains blocked by remaining `Partial` and `Missing` matrix items.


Additional resolved-MSAA observability audit:
- Public support/export payloads now carry `resolved_msaa_compatible`, and aggregate helpers count resolved-MSAA import/export paths. This removes ambiguity between the implemented resolved-payload compatibility path and still-open native sample-level MSAA graph execution.


Additional surface-adjacent promotion audit:
- Headless/stub `RenderTarget::MainSurface` graph extension exports now have focused promotion coverage. This is still not native swapchain image export/promotion.

Additional frame tooling audit:
- Resolved-MSAA public graph export observability now reaches frame stats, profile/capture payloads, debug reports, and capture resource dumps. The focused `msaa` filter validates the full propagation path.

Profile validation note:
- `FrameProfile` propagation for resolved-MSAA public graph export metadata is now explicitly tested, not only compiled.

Additional helper parity audit:
- Resolved-MSAA public graph export helper coverage is now consistent across `FrameStats`, `FrameProfile`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`.

Additional graph-region helper audit:
- Direct `graph_region_export_support` now exposes resolved-MSAA region export counts, closing the helper parity gap with aggregate `graph_import_support`.

## 2026-05-20 - Window usecase graph export audit

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- The public window usecase can now opt into graph extension exports while rendering `RenderTarget::MainSurface` through `Renderer::with_surface`.
- Smoke stats can require and print promoted public graph exports, making the surface-adjacent frame graph promotion path user-visible from an example.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe check -p render_facade_window_usecase` passed.

Still open:
- A real GUI smoke launch with `--require-graph-export` has not been run in this slice.
- Readback-backed surface graph export promotion is implemented; direct swapchain image graph export remains a platform/wgpu capability-gated boundary.
- Full renderer-layer completion remains blocked by remaining `Partial` and `Missing` matrix items.

GUI smoke validation:
- The window usecase was launched with `--graph-export --require-graph-export` and exited successfully after 8 frames.
- Runtime stats confirmed two public graph exports and two promoted public graph exports on the MainSurface window path.

Combined GUI smoke validation:
- The window usecase passed with both `--require-surface-readback` and `--require-graph-export` enabled.
- Runtime stats confirmed one backend surface public frame output and two promoted public graph exports in the same smoke run.

MainColor graph export audit:
- The window usecase now promotes both an extension-owned texture and the standard frame `main_color` graph resource while also materializing the backend surface public frame output.
- The local smoke run passed with three promoted public graph exports and one backend surface public output.
- Native swapchain image export/promotion remains open; this is graph `main_color` promotion, not direct swapchain resource export.

MainColor regression test:
- Added focused unit coverage for exporting `ctx.main_color()` through a public graph extension and promoting it to a public texture handle.
- `render_graph_extension_exports` focused test filter passed with 6 tests.

MainDepth graph export audit:
- Standard frame context exports now cover both main color and main depth resources, including durable public Depth32Float promotion.
- The window MainSurface smoke run promoted main color, main depth, an extension-owned texture, and an extension-owned buffer while also producing a backend surface public frame output.
- Native swapchain image graph export remains open.

Strict graph-export smoke gate:
- The window usecase no longer treats any promoted graph export as sufficient for `--require-graph-export`; it now requires the exact main-color, main-depth, extension texture, and extension buffer promoted outputs.
- The stricter combined GUI smoke run passed.

## 2026-05-20 - Safe graph texture descriptor gate audit

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- New graph code can call `try_create_texture_from_desc` to avoid silent descriptor shape projection when creating graph transient textures from public `TextureDesc` values.
- Array/layered, mipped, and multisampled graph-created texture descriptors now have a focused public validation path with `RenderGraphValidation` errors.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed, 1 passed.

Still open:
- Native graph-created multi-mip/layer/depth/MSAA texture execution remains unimplemented.
- Existing compatibility helper `create_texture_from_desc` remains as the legacy D2 projection path for current callers.
- Full renderer-layer completion remains blocked by remaining `Partial` and `Missing` matrix items.

Additional API safety audit:
- The silent descriptor projection helper is now explicitly deprecated and documented as legacy; the focused builder tests use the explicit validation entry point.

## 2026-05-20 audit update: cross-layer partial region execution

The previous matrix note that cross-layer arbitrary partial-layer flattened region execution/readback was still separate is stale. Current focused evidence shows the public imported texture path is implemented for headless/RHI and backend-wgpu RGBA8:

- `cargo test -p engine_renderer cross_layer -- --nocapture` passed, 2 passed.

This does not close the complete renderer goal because native graph-created multi-shape resources, native swapchain graph export promotion, native MSAA graph execution, and persistent backend-resident graph synchronization remain open.

## 2026-05-20 audit update: persistent graph import cache eviction

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- Persistent backend-wgpu graph import caches for public texture and buffer imports are now tied to public resource lifetime.
- `Renderer::destroy()` removes matching graph RHI texture and buffer import cache entries after successful public texture or buffer destruction.
- Added `destroying_public_graph_import_resources_evicts_persistent_import_cache` to cover cache population followed by public texture and buffer destruction.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer destroying_public_graph_import_resources_evicts_persistent_import_cache -- --nocapture` passed, 1 passed.

Still open:
- Native surface/swapchain graph export promotion.
- custom MSAA resolve validation evidence and custom resolves.

## 2026-05-20 audit update: readback-backed surface main-color graph export promotion

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- Backend surface public frame output materialization can now replace an exported standard-frame `main_color` graph texture handle with the durable backend surface readback texture.
- `RendererGraphExportSource` now distinguishes `BackendMainSurfaceReadback` and `BackendSurfaceReadback` provenance from ordinary `PromotedTransient` graph exports.
- Added `backend_surface_readback_replaces_main_color_graph_export_handle` for the remapping path.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_surface_readback_replaces_main_color_graph_export_handle -- --nocapture` passed, 1 passed.

Still open:
- Direct/non-readback surface or swapchain graph export coverage for readback-disabled, readback-unavailable, or unsupported surface paths.
- custom MSAA resolve validation evidence and custom resolves.

## 2026-05-20 audit update: RHI graphics pipeline MSAA sample counts

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- `RhiGraphicsPipelineDesc` now includes `sample_count`.
- Backend-wgpu maps the RHI graphics pipeline sample count to native `wgpu::MultisampleState`.
- Headless and backend-wgpu render-pass validation compare graphics pipeline sample count with color/depth target sample counts before encoding.
- Added focused headless and backend-wgpu tests for matching MSAA render targets and sample-count mismatch rejection.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_graphics_pipeline_sample_count -- --nocapture` passed, 2 passed.

Still open:
- Explicit per-sample texture access and programmable/custom resolve APIs at the graph/RHI layer.
- Direct/non-readback surface or swapchain graph export coverage for readback-disabled, readback-unavailable, or unsupported surface paths.

## 2026-05-20 audit update: explicit RGBA8 MSAA resolve API

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- `RhiDevice::resolve_texture_rgba8(source, target)` validates and resolves multisampled RGBA8 render attachments into same-sized single-sample RGBA8 render attachments.
- Backend-wgpu uses a native render-pass resolve target for the operation.
- Headless RHI exposes deterministic resolved payload copy for semantic validation.
- `PassContext::resolve_rhi_texture_rgba8(source, target)` makes the resolve callable from RenderGraph pass callbacks.
- Added focused headless and backend-wgpu tests for explicit RGBA8 MSAA resolve.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_resolves -- --nocapture` passed, 8 passed.

Still open:
- Shader-defined custom resolve filters and explicit per-sample texture access.
- Direct/non-readback surface or swapchain graph export coverage for readback-disabled, readback-unavailable, or unsupported surface paths.

## 2026-05-20 audit update: indexed-sample custom RGBA8 MSAA resolve

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- `RhiResolveMode` exposes `Average`, `FirstSample`, and `Sample(u32)` modes.
- `RhiDevice::resolve_texture_rgba8_with_mode()` and `PassContext::resolve_rhi_texture_rgba8_with_mode()` expose mode-selectable resolve through RHI and RenderGraph pass callbacks.
- Backend-wgpu implements indexed-sample resolve as a compute shader over `texture_multisampled_2d<f32>` and RGBA8 storage texture output.
- Headless and backend-wgpu validate out-of-range sample-index requests.
- Added focused RHI and RenderGraph tests for first-sample and non-zero indexed-sample resolve.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_resolves -- --nocapture` passed, 8 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 3 passed.

Still open:
- User-supplied shader resolve kernels beyond the built-in indexed-sample resolve modes.
- Direct/non-readback surface or swapchain graph export coverage for readback-disabled, readback-unavailable, or unsupported surface paths.

## 2026-05-20 audit update: backend-wgpu custom WGSL RGBA8 MSAA resolve shader

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- `RhiResolveShaderDesc` and `RhiDevice::resolve_texture_rgba8_with_shader()` expose a user-supplied WGSL custom resolve path for backend-wgpu.
- The shader ABI binds the multisampled RGBA8 source at group 0 binding 0 and the single-sample RGBA8 storage target at group 0 binding 1.
- Backend-wgpu validates resolve source/target shape and usage, creates the compute pipeline, binds source/target texture views, and dispatches over the output extent.
- `PassContext::resolve_rhi_texture_rgba8_with_shader()` has a backend-wgpu RenderGraph test that renders an MSAA source, runs a custom WGSL resolve pass, and exports the resolved target.
- Headless RHI explicitly returns `UnsupportedFeature(BackendWgpu)` for the WGSL shader path.
- Added focused headless and backend-wgpu tests for the shader path.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_resolves -- --nocapture` passed, 8 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 3 passed.

Still open:
- Broader custom resolve shader ABI coverage and non-RGBA8/custom format support.
- Direct/non-readback surface or swapchain graph export coverage for readback-disabled, readback-unavailable, or unsupported surface paths.

## 2026-05-20 audit update: backend-wgpu custom WGSL RGBA16F MSAA resolve shader

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- `RhiDevice::resolve_texture_rgba16f_with_shader()` and `PassContext::resolve_rhi_texture_rgba16f_with_shader()` extend custom WGSL MSAA resolve to RGBA16F/HDR textures.
- Backend-wgpu validates multisampled RGBA16F sampled source textures and single-sample RGBA16F storage targets, creates the compute pipeline, and dispatches over the output extent.
- Added a focused backend-wgpu test that validates the half-float output payload.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_resolves -- --nocapture` passed, 8 passed.

Still open:
- RGBA32F, depth, and sRGB/BGRA 8-bit custom resolve coverage at this checkpoint.
- Direct/non-readback surface or swapchain graph export coverage for readback-disabled, readback-unavailable, or unsupported surface paths.

## 2026-05-20 audit update: backend-wgpu custom WGSL RGBA32F MSAA resolve shader

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- `RhiDevice::resolve_texture_rgba32f_with_shader()` and `PassContext::resolve_rhi_texture_rgba32f_with_shader()` extend custom WGSL MSAA resolve to RGBA32F textures.
- Backend-wgpu validates multisampled RGBA32F sampled source textures and single-sample RGBA32F storage targets, creates the compute pipeline, and dispatches over the output extent.
- Added a focused backend-wgpu test that validates the float output payload.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_resolves -- --nocapture` passed, 8 passed. The RGBA32F native execution branch is gated on wgpu guaranteed-format MSAA support, so unsupported adapters validate the capability branch instead of entering invalid native texture creation.

Still open:
- Depth and sRGB/BGRA 8-bit custom resolve coverage at this checkpoint.
- Direct/non-readback surface or swapchain graph export coverage for readback-disabled, readback-unavailable, or unsupported surface paths.

## 2026-05-20 audit update: surface graph export unsupported provenance

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- Backend surface `main_color` graph exports are no longer reported as successful promoted outputs when backend surface readback cannot materialize a durable public frame output.
- Unsupported surface graph exports are marked with `BackendSurfaceReadbackUnsupported`, `BackendSurfaceReadbackDisabled`, or `BackendSurfaceReadbackUnavailable` provenance and `promoted = false`.
- Imported-public graph export counts now key off `RendererGraphExportSource::ImportedPublic`, so unsupported surface exports are not misclassified as imports.
- Added a focused test for the readback-disabled provenance path.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_surface_readback_unsupported_marks_main_color_graph_export_unpromoted -- --nocapture` passed, 1 passed.

Still open:
- Depth and sRGB/BGRA 8-bit custom resolve coverage at this checkpoint.
- Platform-specific direct swapchain export mechanisms beyond readback-backed materialization or explicit unsupported provenance.

## 2026-05-20 audit update: surface graph export support query

Result: improved but renderer goal remains incomplete.

Closed in this slice:
- `Renderer::surface_graph_export_support()` exposes direct swapchain graph export support separately from readback-backed surface graph export support and enabled state.
- Current renderer paths explicitly report direct swapchain image graph export as unsupported and provide a tooling-facing reason.
- Added focused support-query coverage for the unsupported direct path.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer surface_graph_export_support_reports_direct_swapchain_export_unsupported -- --nocapture` passed, 1 passed.

Still open:
- Depth and sRGB/BGRA 8-bit custom resolve coverage at this checkpoint.
- Any future platform-specific direct swapchain export mechanism beyond readback-backed materialization plus explicit unsupported provenance.

## 2026-05-20 audit update: focused validation sweep and stabilization

Result: focused MSAA resolve, graph export, and import-cache evidence is now verified, but renderer goal remains incomplete.

Closed or stabilized in this slice:
- `GraphTextureRendererDesc` preserves `TextureDesc::usage`, and graph-created RHI textures combine descriptor usage with graph access-derived usage. This fixes custom resolve targets that require storage usage.
- Backend-wgpu graph execution submits pending encoded graph command buffers before callbacks that may issue immediate RHI work, so custom resolve callbacks can observe earlier graph writes.
- Backend-wgpu RHI texture creation validates MSAA texture requirements before native texture creation, including render-attachment usage and guaranteed format sample-count support.
- RGBA32F MSAA custom resolve coverage is now capability-gated through wgpu guaranteed format features instead of assuming every adapter exposes the native MSAA format path.
- Environment bake packed mip-chain textures clear stale base-level layout metadata when marked as generated mips.
- Internally materialized public frame output textures no longer add fake pending upload work to upload queue stats.
- The game-layer prelude boundary test now checks exact identifiers, so public renderer graph-export names do not create false positives while low-level graph/RHI details stay out of the prelude.
- The public graph execution failure regression now uses an actual invalid graph export handle, preserving stale `last_graph_execution` clearing without relying on now-supported D2Array import behavior.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_resolves -- --nocapture` passed, 10 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_graphics_pipeline_sample_count -- --nocapture` passed, 2 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 4 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_ -- --nocapture` passed, 133 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu -- --test-threads=1` passed, 41 unit tests plus 1 integration test plus doc-tests.

Validation caveat:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer` with the default parallel harness exited with Windows `STATUS_ACCESS_VIOLATION` after many backend-wgpu tests. The serial full-suite run passed and is the authoritative full-crate evidence for this pass.

Still open:
- direct native surface/swapchain graph export capability gate remains incomplete beyond readback-backed materialization and explicit unsupported provenance.
- Backend fence objects, true nonblocking per-submission completion queries, and remaining backend-owned tombstone coverage remain `Partial` in the matrix.

## 2026-05-20 audit update: Depth32F custom MSAA resolve shader

Result: Depth32F custom resolve coverage is now implemented and verified, but renderer goal remains incomplete.

Closed in this slice:
- `RhiDevice::resolve_texture_depth32f_with_shader()` exposes a backend-wgpu custom depth resolve path from multisampled Depth32Float source textures to single-sample Depth32Float render-attachment targets.
- The backend-wgpu shader ABI binds the source as `texture_depth_multisampled_2d` at group 0 binding 0 and runs the caller fragment entry over a fullscreen pass that writes `@builtin(frag_depth)`.
- `PassContext::resolve_rhi_texture_depth32f_with_shader()` exposes the same operation to RenderGraph callbacks.
- RHI graphics pipeline validation now accepts depth-only fragment pipelines with no color target.
- Headless RHI explicitly reports the depth shader path as `UnsupportedFeature(BackendWgpu)`.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_resolves -- --nocapture` passed, 10 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 4 passed.

Still open:
- direct native surface/swapchain graph export capability gate remains incomplete beyond readback-backed materialization and explicit unsupported provenance.
- Backend fence objects, true nonblocking per-submission completion queries, and remaining backend-owned tombstone coverage remain `Partial` in the matrix.

## 2026-05-20 audit update: 8-bit sRGB/BGRA custom MSAA resolve shader

Result: current public `TextureFormat` custom resolve coverage is now implemented and verified, but renderer goal remains incomplete.

Closed in this slice:
- `RhiDevice::resolve_texture_8bit_color_with_shader()` exposes a backend-wgpu custom fragment resolve path for multisampled `Rgba8UnormSrgb` and `Bgra8UnormSrgb` textures.
- The backend-wgpu shader ABI binds the source as `texture_multisampled_2d<f32>` at group 0 binding 0 and runs the caller fragment entry over a fullscreen pass that writes the single-sample target as a color render attachment.
- `PassContext::resolve_rhi_texture_8bit_color_with_shader()` exposes the same operation to RenderGraph callbacks.
- The path avoids storage-texture requirements for sRGB/BGRA target formats by using a render pass instead of the compute/storage ABI used by the existing `Rgba8Unorm` custom resolve.
- Headless RHI explicitly reports the fragment shader path as `UnsupportedFeature(BackendWgpu)`.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer srgb_msaa_texture_with_custom_fragment_shader -- --nocapture` passed, 3 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_resolves -- --nocapture` passed, 10 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 4 passed.

Still open:
- direct native surface/swapchain graph export capability gate remains incomplete beyond readback-backed materialization and explicit unsupported provenance.
- Backend fence objects, true nonblocking per-submission completion queries, and remaining backend-owned tombstone coverage remain `Partial` in the matrix.

## 2026-05-20 audit note: custom MSAA resolve matrix

Prompt-to-artifact status for custom MSAA resolve coverage is updated: `rhi.rs` now has `RhiCustomResolveSupport`, per-path support records, and `RhiDevice::custom_resolve_support()` implementations for headless and backend-wgpu. `graph.rs` exposes the same capability through `PassContext::rhi_custom_resolve_support()`. This closes the previous weak spot where custom resolve format support had to be inferred from individual API methods.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.




## 2026-05-20 audit note: cooperative background retirement startup

Prompt-to-artifact status for GPU memory/upload/delayed destroy improved: `RendererFeature::BackgroundResourceRetirement` now maps to a supported cooperative startup path instead of `UnsupportedFeature`. Active state is observable from the renderer, memory stats, and explicit retirement stats. `RendererFeature::NonblockingResourceRetirementPoll` remains a real capability gate because current wgpu completion observability still falls back to queue-empty polling.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer background_resource_retirement -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer renderer_feature -- --nocapture` passed, 4 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

Residual risk: direct cross-thread renderer/wgpu mutation and true nonblocking backend submission-index completion queries remain open matrix items.


Implementation refinement: the background retirement path now includes a scheduler-thread lifecycle. The thread does not mutate renderer or wgpu objects; it requests ticks that are consumed by renderer-thread safe points. This keeps thread safety clear while closing the previous unsupported-only startup API.

## 2026-05-20 audit update: backend-wgpu native pipeline replacement tombstones

Result: backend-owned tombstone coverage improved, but renderer goal remains incomplete.

Closed in this slice:
- Replacing backend-wgpu native reflected pipeline objects for an existing `PipelineKey` now tombstones the previous object instead of dropping it immediately.
- The tombstone retains the old shader module, layout objects, material bind groups, owned uniform buffers, render-pipeline reference, and backend fence metadata.
- The structural native render-pipeline cache entry stays live when another current object, including the replacement, still references the same render-pipeline key.
- `BackendResourceRetirementStats` exposes live and retired replacement tombstone counts for native pipeline entries, render-pipeline refs, shader modules, bind groups, owned buffers, and fence objects.

Evidence:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_native_pipeline_replacement_enters_backend_tombstone -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_native_cache_reuses_render_pipeline_across_material_bind_groups -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_wgpu::tests -- --test-threads=1` passed, 41 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

Still open:
- Direct native surface/swapchain graph export remains incomplete beyond readback-backed materialization and explicit unsupported provenance.
- True backend fence objects/nonblocking per-submission completion queries and any backend-owned resource classes not yet represented by tombstones remain `Partial` in the matrix.

## 2026-05-20 audit note: pipeline cache backend coverage

Prompt-to-artifact status for pipeline cache improved: `PipelineCacheBackendCoverage` maps every facade pipeline entry to backend-object coverage evidence, including missing keys. The renderer also synchronizes facade entry backend-object IDs from active backend-wgpu native pipeline objects before recomputing cache usage stats. Full backend-native pipeline cache completion remains open until real rendering paths produce backend objects for every facade-ready entry.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer pipeline -- --nocapture` passed, 18 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

## 2026-05-20 audit note: post-process backend coverage artifact

Prompt-to-artifact status for post-process coverage improved: frame stats, debug reports, and captures can now compute `FramePostProcessBackendCoverage` from declared post-process outputs and backend RHI/native labels. Dynamic combined backend labels are mapped to semantic pass labels, and uncovered custom or missing outputs remain explicitly listed.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer post_process_backend_coverage -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests. Residual risk: sampled branches remain minimal renderer implementations rather than production-grade effect pipelines.

## 2026-05-20 audit note: post-process support matrix

Prompt-to-artifact status for post-process improved: `Renderer::post_process_support()` exposes per-effect backend visibility, implementation level, label token, production readiness, and limitation text. Backend-visible sampled branches are no longer conflated with production-complete bloom/TAA/SSR/DOF/motion-blur/HDR/color-grading pipelines.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer post_process_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests. Residual risk: production-grade post-process resource chains remain open by design and are queryable as per-effect limitations.

## 2026-05-20 audit note: deformation support matrix

Prompt-to-artifact status for deformation improved: `Renderer::deformation_support()` maps each deformation-related feature to support state, implementation level, and limitation. Backend GPU deformation remains an explicit unsupported backend path, while facade/graph observable animation outputs are no longer hidden behind one broad Partial entry.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer deformation_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests. Residual risk: true backend GPU deformation buffers and draw submission remain open.

## 2026-05-20 audit note: lighting and IBL support matrix

Prompt-to-artifact status for light/shadow/environment improved: `Renderer::lighting_support()` maps each lighting-related capability to support state, implementation level, and limitation text. Backend IBL convolution and runtime environment capture remain explicit unsupported backend paths, while retained/facade and graph-observable lighting outputs are no longer hidden behind one broad Partial entry.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer lighting_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests. Residual risk: backend IBL convolution/capture execution remains open.

## 2026-05-20 audit note: frame capture support matrix

Prompt-to-artifact status for frame capture improved: `Renderer::frame_capture_support()` maps internal capture, external hook handoff, native SDK blockers, and unavailable backends into one public artifact. Native RenderDoc/external-debugger SDK integration remains an external blocker; registered hooks remain the current integration point.
Frame-capture lifecycle now also surfaces a clearer execution outcome: successful external callback invocation now records `FrameCaptureStatus::Captured` in the finished `FrameCapture`, while panic callbacks remain `BackendHookFailed` and hook removal still resolves to `BackendUnavailable`.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_capture_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests. Residual risk: native SDK loading and begin/end capture calls remain open.

## 2026-05-20 audit note: debug tooling support matrix

Prompt-to-artifact status for debug/editor tooling improved: `Renderer::debug_tooling_support()` maps debug draw, picking, frame debug report, frame capture, and native frame debugger capture to support state, implementation level, and limitation text. Native debugger SDK capture remains an explicit unsupported external integration point.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer debug_tooling_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests. Residual risk: live editor UI integration and native debugger SDK calls remain open.

## 2026-05-20 audit note: resource lifecycle support matrix

Prompt-to-artifact status for resource lifecycle improved: `Renderer::resource_lifecycle_support()` maps renderer resource classes to lifecycle, stale-handle, upload/readback, residency, observability, backend residency, and limitation evidence. Backend-wgpu persistent resource synchronization remains a visible partial backend-residency gap.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer resource_lifecycle_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests. Residual risk: complete backend-resident dirty synchronization remains open.

## 2026-05-20 audit note: backend synchronization support matrix

Prompt-to-artifact status for backend synchronization improved: `Renderer::backend_synchronization_support()` maps synchronization/retirement features to support state, implementation level, active background scheduler state, and limitation text. Queue-empty fallback and scheduler-thread requests are no longer conflated with true nonblocking per-submission completion polling.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_synchronization_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests. Residual risk: true nonblocking backend submission-index completion query remains open.

## 2026-05-20 audit update: RenderGraph support query is evidence, not completion

The renderer facade now has a public `RendererRenderGraphSupport` report exposed through `Renderer::render_graph_support()`. The audit treats this as a useful boundary artifact because it separates implemented graph import/export and promotion paths from backend/runtime-dependent or unsupported graph capabilities.

Current audit conclusion for this slice:

- Implemented evidence: public support query type, per-capability entries, unsupported capability listing, headless boundary test added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 tests plus doc-tests. Residual gap: the query does not itself implement direct swapchain graph export or complete backend-resident synchronization.
- Goal status: still open. A support matrix is not a substitute for full renderer execution paths.

## 2026-05-20 audit update: backend material dependency invalidation

The audit now records a real backend lifecycle improvement: material-bound texture/sampler dependency invalidation is wired into renderer mutations and destruction. Old backend-wgpu material bindings are explicitly unregistered and affected native pipeline objects are invalidated so backend objects retire through tombstones.

Current audit conclusion for this slice:

- Implemented evidence: texture update/generate-mips/destroy invalidation hook, sampler destroy invalidation hook, material destroy/removal/replacement invalidation hook, dependency lookup test added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 tests plus doc-tests, including `material_dependency_lookup_tracks_texture_and_sampler_users`. Residual gap: this does not cover persistent buffer dirty-range synchronization or full multi-subresource native texture residency.
- Goal status: still open.

## 2026-05-20 audit update: backend material binding stats

The renderer now exposes material-bound backend resource registry counts. The audit treats this as observability evidence for backend material texture/sampler lifecycle work, not as complete resource lifecycle closure.

Current audit conclusion for this slice:

- Implemented evidence: backend-wgpu registry stats, public renderer stats, prelude export, headless inactive test added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_material_resource_stats -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 tests plus doc-tests. Residual gap: the stats do not prove complete dirty synchronization across textures, buffers, meshes, render targets, or graph resources.
- Goal status: still open.

## 2026-05-20 audit update: frame/debug/capture propagation for backend material stats

The audit now records that backend material resource stats are propagated through frame/debug/capture outputs, not only available through a standalone renderer query. This strengthens the evidence for material-bound backend resource lifecycle observability.

Current audit conclusion for this slice:

- Implemented evidence: `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` carry `BackendMaterialResourceStats`; frame instrumentation fills it; propagation test added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_preserves_backend_material_resource_stats -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 tests plus doc-tests. Residual gap: the stats still cover only material external texture/sampler bindings.
- Goal status: still open.

## 2026-05-20 audit update: material backend support matrix

The audit now records the material backend boundary as a first-class renderer query. This prevents facade material support, reflected wgpu custom-material support, and complete dynamic material-template backend integration from being conflated.

Current audit conclusion for this slice:

- Implemented evidence: public `MaterialBackendSupport` matrix, backend active flag, per-feature implementation level, unsupported feature listing, reflected-backend aggregate helper, and dynamic-template backend aggregate helper.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer material_backend_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 tests plus doc-tests.
- Residual gap: the matrix does not implement complete dynamic material-template backend pipeline layouts/bind groups; it makes that boundary explicit.
- Goal status: still open.

## 2026-05-20 audit update: graph RHI import cache dirty-state stats

The audit now records a public dirty-state report for persistent graph RHI import caches. The renderer can expose stale public texture/buffer revisions in the graph import cache without disabling cache reuse.

Current audit conclusion for this slice:

- Implemented evidence: public cache stats type, renderer query, stale revision test added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_rhi_import_cache_stats -- --nocapture` passed, 2 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 tests plus doc-tests. Residual gap: this observes dirty state but does not by itself prove every backend resource class has complete dirty synchronization.
- Goal status: still open.

## 2026-05-20 audit update: frame/debug/capture propagation for graph import cache stats

The audit now records that graph RHI import cache dirty-state stats propagate through frame/debug/capture outputs. This turns persistent graph import cache synchronization state into a standard renderer-observable artifact instead of a standalone query only.

Current audit conclusion for this slice:

- Implemented evidence: `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` carry `RendererGraphRhiImportCacheStats`; frame instrumentation fills it; propagation test added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_preserves_graph_rhi_import_cache_stats -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_rhi_import_cache_stats -- --nocapture` passed, 2 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 tests plus doc-tests. Residual gap: this reports graph import cache synchronization state without completing every backend residency path.
- Goal status: still open.

## 2026-05-20 audit update: graph import cache dirty footprint accounting

The audit now records that graph RHI import cache dirty-state stats quantify the amount of stale public resource data, not only whether stale entries exist. This strengthens evidence for buffer dirty-range synchronization observability.

Current audit conclusion for this slice:

- Implemented evidence: stale texture byte count, stale buffer represented range count, stale buffer byte count, aggregate stale byte count, and updated focused test expectations.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_rhi_import_cache_stats -- --nocapture` passed, 2 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 tests plus doc-tests. Residual gap: this is still observability and does not prove every backend residency path is complete.
- Goal status: still open.

## 2026-05-20 audit update: frame/debug/capture propagation for pipeline cache backend coverage

The audit now records that `PipelineCacheBackendCoverage` propagates through frame/debug/capture outputs. This makes facade/backend pipeline object coverage available to renderer tooling without requiring a separate query.

Current audit conclusion for this slice:

- Implemented evidence: `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` carry `PipelineCacheBackendCoverage`; frame instrumentation fills it; propagation test added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_preserves_pipeline_cache_backend_coverage -- --nocapture` passed, 1 passed. Residual gap: propagation does not by itself make backend pipeline cache coverage complete.
- Goal status: still open.

## 2026-05-20 audit update: pipeline cache missing backend object classification

The audit now records more precise diagnostics for pipeline cache backend coverage. Missing backend object entries are split into ready, used, and unused categories so tooling can identify whether missing backend coverage affected the current frame.

Current audit conclusion for this slice:

- Implemented evidence: added ready/unused missing backend object counters and updated focused coverage expectations.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer pipeline_warmup_validates_pipeline_keys -- --nocapture` passed, 1 passed. Residual gap: this remains diagnostic coverage and does not complete backend pipeline cache implementation.
- Goal status: still open.

## 2026-05-20 audit update: sampler info and destroyed texture-view output coverage

The audit now records a small public sampler inspection closure plus a specialized destroyed frame-output target test. `Renderer::sampler_info()` gives tools live sampler descriptor/status data, and destroyed sampler state is explicitly left to `Renderer::resource_status()` once the retained descriptor payload is gone. Public texture-view frame output now has focused coverage for destroyed texture targets before writeback.

Current audit conclusion for this slice:

- Implemented evidence: `SamplerInfo`, `Renderer::sampler_info()`, prelude export, and sampler info lifecycle test added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer sampler_info_reports_desc_status_and_destroyed_payload_boundary -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer texture_view_frame_output_rejects_destroyed_target_texture -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.
- Residual gap: this does not complete backend-resident texture/sampler synchronization or broaden backend GPU mip generation beyond the existing material texture path.
- Goal status: still open.

## 2026-05-20 audit update: backend submission completion report

The audit now records a structured completion report for backend submission polling. This prevents queue-empty fallback behavior from being mistaken for true nonblocking per-submission completion support.

Current audit conclusion for this slice:

- Implemented evidence: public completion report, renderer query, frame/debug/capture propagation, focused tests added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `backend_submission_completion_report_exposes_nonblocking_limit` and `frame_debug_report_preserves_backend_submission_completion_report`. Residual gap: true nonblocking per-submission backend completion remains unsupported.
- Goal status: still open.

## 2026-05-20 audit update: backend completion report in resource dumps

The audit now records that resource dumps carry the backend submission completion report. This removes a propagation gap between capture payloads and resource dump payloads.

Current audit conclusion for this slice:

- Implemented evidence: resource dump field added and focused propagation assertion extended.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `frame_debug_report_preserves_backend_submission_completion_report`. Residual gap: the report still documents an unsupported true nonblocking completion path.
- Goal status: still open.

## 2026-05-20 audit update: backend completion report in retirement stats

The audit now records that `ResourceRetirementStats` carries the backend submission completion report. This aligns the resource-retirement API with frame/debug/capture/resource-dump observability.

Current audit conclusion for this slice:

- Implemented evidence: retirement stats field added and focused propagation test added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `resource_retirement_stats_preserve_backend_submission_completion_report`. Residual gap: true nonblocking per-submission backend completion remains unsupported.
- Goal status: still open.

## 2026-05-20 audit update: backend completion report tombstone counters

The audit now records tombstone wait/retire pressure in the backend completion report. This improves the ability to diagnose whether backend resource retirement is blocked on submission-index coverage or queue-empty fallback.

Current audit conclusion for this slice:

- Implemented evidence: report fields added, renderer mapping added, focused expectations updated.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `backend_submission_completion_report_exposes_nonblocking_limit`. Residual gap: true nonblocking per-submission backend completion remains unsupported.
- Goal status: still open.

## 2026-05-20 audit update: external render target destroyed attachment validation

The audit now records explicit frame-time coverage for external render targets whose attachment resources have been destroyed after target creation. The renderer rejects destroyed color and depth textures before rendering or public output materialization.

Current audit conclusion for this slice:

- Implemented evidence: frame-time external render target descriptor validation already rechecks attachment liveness; focused destroyed color/depth attachment tests added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer external_render_target_rejects_destroyed_attachment_at_frame_time -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer external_render_target_rejects_destroyed_depth_attachment_at_frame_time -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.
- Residual gap: this is a specialized stale-resource coverage closure, not backend-resident synchronization completion.
- Goal status: still open.

## 2026-05-20 audit update: explicit nonblocking backend completion error path

The audit now records a public callable error path for true nonblocking backend completion polling. The API makes the missing capability explicit and user-visible instead of only represented by report fields.

Current audit conclusion for this slice:

- Implemented evidence: public API added and focused validation-error test added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `nonblocking_backend_submission_completion_poll_reports_user_visible_error`. Residual gap: the real nonblocking backend completion query is still unsupported.
- Goal status: still open.

## 2026-05-20 audit update: explicit direct swapchain graph export gate

The audit now records a public callable error path for direct swapchain graph export. This prevents unsupported direct swapchain export from being represented only by broad support-matrix wording.

Current audit conclusion for this slice:

- Implemented evidence: public gate added and focused validation-error test added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `direct_swapchain_graph_export_gate_returns_user_visible_error`. Residual gap: native direct swapchain image export is still unsupported.
- Goal status: still open.

## 2026-05-20 audit update: surface graph export support propagation

The audit now records that surface graph export support state propagates through frame/debug/capture/resource-dump outputs. This removes a propagation gap for direct swapchain graph export support and readback-backed surface graph export support.

Current audit conclusion for this slice:

- Implemented evidence: frame/debug/capture/resource dump fields added and propagation test added.
- Validation evidence: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `frame_debug_report_preserves_surface_graph_export_support`. Residual gap: native direct swapchain image graph export remains unsupported.
- Goal status: still open.

## 2026-05-20 audit update: RenderGraph support matrix propagation

The audit now records that the RenderGraph support matrix propagates through frame/debug/capture/resource-dump outputs. This makes unsupported graph capabilities visible in ordinary renderer observability artifacts.

Current audit conclusion for this slice:

- Implemented evidence: frame/debug/capture/resource dump fields added, default support matrix added, focused propagation test added.
- Weak or incomplete evidence: tests were not run in this pass, and native direct swapchain graph export remains unsupported.
- Goal status: still open.

## 2026-05-21 execution note: surface runtime consistency checks and completion tracker boundaries

The audit now records renderer-side consistency and runtime-boundary behavior for surface creation, plus explicit tracker-gated nonblocking completion semantics.

Current audit conclusion for this slice:

- Implemented evidence: window/display handle validation in `Renderer::with_surface`, runtime-format consistency validation for configured surface/depth formats, and completion-index tracker reuse for repeated submission indexes.
- Validation behavior evidence: `Renderer::poll_backend_submission_completion_nonblocking()` now reports user-visible validation when no tracker exists and succeeds when a tracker-backed completion path is available.
- Coverage evidence: tests added for `with_surface_validates_window_handles_for_surface_creation`, `with_surface_requires_backend_wgpu_if_unavailable`, `validate_surface_runtime_formats_rejects_configured_color_format_mismatch`, `validate_surface_runtime_formats_rejects_configured_depth_format_mismatch`, `nonblocking_backend_submission_completion_poll_can_be_supported_after_real_submission`, `nonblocking_backend_submission_completion_poll_reports_user_visible_error_without_trackers`, `feature_support_reflects_nonblocking_completion_tracker_state`, and `wgpu_submission_fence_reuses_tracker_for_repeated_same_submission_index`.
- Residual gap: direct native swapchain image graph export, full backend residency synchronization, broader native direct texture/graph synchronization, and production-complete standard renderer paths remain open.
- Goal status: still open.

- `cargo test -p engine_renderer with_surface_validates_display_handles_for_surface_creation -- --nocapture`
  - Result: passed, including display-handle validation for `Renderer::with_surface` through a `HasWindowHandle`-valid/`HasDisplayHandle`-unavailable window test stub, and asserting `RendererError::Validation` contains `display` when creating a surface with an unavailable display handle.

### 本轮 Window/surface 错误语义补充

- `Renderer::with_surface` 增加了独立的 display 句柄校验回归：窗口仅提供有效窗口句柄但缺少可用 display 时，应返回 validation 错误而不是继续尝试 surface 创建。
- 该场景通过新增测试桩对象 `DummySurfaceWindowWithoutDisplay` 覆盖，避免了与现有窗口句柄缺失路径混淆。

- `cargo test -p engine_renderer with_surface_invokes_window_handle_validation_before_display_validation -- --nocapture`
  - Result: passed, including explicit ordering/count validation that `window_handle()` is called before `display_handle()` and both are invoked once when display validation fails after a successful window handle check.

- `cargo test -p engine_renderer with_surface_short_circuits_display_validation_on_window_handle_error -- --nocapture`
  - Result: passed, including a short-circuit regression ensuring `Renderer::with_surface` returns window-handle validation errors without querying display handles; display queries were tracked and confirmed not invoked when `HasWindowHandle::window_handle()` returns `HandleError::Unavailable`.
