# Renderer goal coverage matrix

Source of truth: `docs/rust_3d_renderer_api_design.md`.

Status meanings:
- `Implemented`: real renderer-layer semantics exist and are covered by tests.
- `Partial`: API and some semantics exist, but a key path, backend behavior, verification, or example is incomplete.
- `Stub`: API/graph/stat shape exists, but behavior is mostly observational or placeholder.
- `Missing`: documented renderer-layer capability is absent.

This matrix is the execution queue for `docs/renderer_goal.md`; it is not a completion claim.

Latest verification in this session:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1`: 408 passed plus doc-tests passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer` using the default parallel test harness hit a Windows `STATUS_ACCESS_VIOLATION` after many backend-wgpu tests; the serial full-suite run above passed and is the current full-crate evidence for this pass.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_resolves -- --nocapture`: 10 passed, covering explicit RGBA8 MSAA resolves, first/sample-index resolves, custom RGBA8/RGBA16F/Depth32F resolve shader paths, RGBA8-sRGB/BGRA8-sRGB fragment custom resolve paths, headless unsupported custom shader behavior, and the cap-gated RGBA32F resolve shader test.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_graphics_pipeline_sample_count -- --nocapture`: 2 passed, covering headless and backend-wgpu RHI MSAA pipeline sample-count validation.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_pass_context_resolves -- --nocapture`: 4 passed, covering RenderGraph callback resolve integration for first-sample, custom color WGSL, sRGB 8-bit custom fragment, and custom Depth32F WGSL resolve paths.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer destroying_public_graph_import_resources_evicts_persistent_import_cache -- --nocapture`: passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_surface_readback_replaces_main_color_graph_export_handle -- --nocapture`: passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_surface_readback_unsupported_marks_main_color_graph_export_unpromoted -- --nocapture`: passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer surface_graph_export_support_reports_direct_swapchain_export_unsupported -- --nocapture`: passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer background_resource_retirement -- --nocapture`: passed, 1 passed, covering cooperative background retirement startup, active-state observability, and memory/retirement stats propagation.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer renderer_feature -- --nocapture`: 4 passed, covering `RendererFeature::BackgroundResourceRetirement` as supported facade semantics, `RendererFeature::NonblockingResourceRetirementPoll` as a tracker-gated runtime capability boundary (supported only while a true completion tracker is active; otherwise config-gated fallback is reported), and the public feature support matrix grouping.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_native_pipeline_replacement_enters_backend_tombstone -- --nocapture`: passed, 1 passed, covering backend-wgpu native pipeline replacement moving old backend objects into tombstones.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_wgpu::tests -- --test-threads=1`: 41 passed, covering backend-wgpu lifetime/cache/tombstone regressions after the replacement-tombstone change.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_ -- --nocapture`: 133 passed, including public graph execution/export/import regression coverage after the RenderGraph/RHI resolve changes.
- `cargo test -p engine_renderer profiler_populates_gpu_time_for_imported_extension_buffers`: passed, including high-level GPU time populated for a facade graph with imported graph-extension buffer resources.
- `cargo test -p engine_renderer profiler_populates_gpu_time_for_imported_extension_textures`: passed, including high-level GPU time populated for a facade graph with imported graph-extension texture resources.
- `cargo test -p engine_renderer profiler_populates_gpu_time_for_imported_environment_textures`: passed, including high-level GPU time populated for a facade graph with imported environment texture resources.
- `cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms`: passed, including native wgpu mesh renderer GPU timestamp stats conversion.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu -- --test-threads=1`: 41 unit tests passed, 1 integration test passed, and doc-tests passed.
- `cargo test -p engine_renderer wgpu_metrics_`: 2 passed, including `render_wgpu` timestamp metrics mapped into high-level `FrameStats` when profiling is enabled, hidden from high-level stats when profiling is disabled, and native wgpu pass labels preserved in `RenderGraphStats::rhi_executed_pass_labels`.
- `cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed, including editor/debug-report exposure of backend-wgpu native pass labels, GPU profiler state/time, draw/visibility counts, reclaim policy, and `PipelineCacheStats::backend_objects`.
- `cargo test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`: passed, covering backend-wgpu render pass label order for directional shadow cascades, spot shadows, point shadow cube faces, and the final mesh pass, plus no-shadow behavior when no items are visible.
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`: passed, covering wgpu facade frame graph-stat merging so facade semantic pass labels/barriers and backend native RHI labels/timestamps/GPU time survive together.
- `cargo test -p engine_renderer initial_gpu_profiler_state_requires_timestamp_capability`: passed, including initial `RendererConfig::gpu_profiling` state gated by timestamp capability.
- `cargo test -p engine_renderer renderer_features_cover_modern_renderer_capability_bits`: passed, including public `RendererFeatures::SURFACE` bit coverage.
- `cargo test -p engine_renderer renderer_feature_info_reports_tiers_and_unsupported_reasons`: passed, including public feature stability tiers, implementation levels, and unsupported reasons for core/optional/experimental/reserved features.
- `cargo test -p engine_renderer renderer_feature_infos_enumerates_all_public_features`: passed, including runtime enumeration of every public `RendererFeature`, `RendererFeatureAudit` aggregation of total/supported/unsupported, core-supported/core-unsupported, unsupported-without-reason, backend-real/facade-semantic/graph-semantic/reserved implementation counts, supported-non-backend-real feature listing, and core/optional/experimental/reserved feature counts, plus `RendererCargoFeatureAudit` enabled/disabled aggregation and classification of all 17 engine_renderer Cargo features.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer renderer_feature_support_matrix -- --nocapture`: passed, 1 passed, covering `Renderer::feature_support_matrix()` grouping by stability tier and implementation level so backend-real, facade-semantic, graph-semantic, config-gated, and reserved support are explicitly separated.
- `cargo test -p engine_renderer renderer_new_selects_configured_backend_without_surface`: passed, including wgpu backend initialization without a surface no longer advertising `RendererFeature::Surface`.
- `cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed, including public `FrameDebugReport` summarizing the last frame for editor/inspector use and exposing the full `RenderGraphStats` snapshot, pass-level `rhi_executed_pass_labels`, profiler state, pipeline/material switches, per-frame `PipelineCacheStats`, upload/memory stats, submission-boundary retirement frame fields, standard frame output lists, capture trigger/request parameters, capture request id, queued-frame index and latency, capture backend integration snapshot, capture external-hook handoff state and hook metadata, and capture label/backend/status/resource dump data from the last completed frame.
- `cargo test -p engine_renderer graph_ -- --nocapture`: 33 passed, including `RenderGraphStats::semantic_passes` for graph/facade-semantic execution, `RenderGraphStats::rhi_executed_passes` / `rhi_executed_pass_labels` for RHI/backend execution, and wgpu-backed graph/RHI execution preserving execution-kind observability.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_promotes_exported_transients_to_public_handles -- --nocapture`: passed, covering backend-wgpu `Renderer::execute_graph_to_resources` promotion of transient graph buffer exports plus D2 RGBA8/RGBA16F/RGBA32F/Depth32Float texture exports into durable public `BufferHandle` / `TextureHandle` resources.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_texture_shapes -- --nocapture`: passed, covering backend-wgpu `Renderer::execute_graph_to_resources` upload/read/writeback for imported public textures across D1, D2, D2Array, D3, Cube, CubeArray RGBA8 plus D2 RGBA16F/RGBA32F/Depth32Float and flattened D2Array Depth32Float.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_buffer_export -- --nocapture`: passed, covering backend-wgpu `Renderer::execute_graph_to_resources` upload/read/writeback for imported public buffer exports and `ImportedPublic` provenance metadata.
- `destroying_public_graph_import_resources_evicts_persistent_import_cache`: passed, covering persistent backend-wgpu graph import cache eviction when imported public texture and buffer handles are destroyed.
- `backend_surface_readback_replaces_main_color_graph_export_handle`: passed, covering backend surface readback public output replacing the durable handle and provenance of a promoted standard-frame `main_color` graph export.
- `headless_rhi_graphics_pipeline_sample_count_matches_render_targets` and `wgpu_rhi_graphics_pipeline_sample_count_matches_msaa_target`: passed through `rhi_graphics_pipeline_sample_count`, covering RHI graphics pipeline `sample_count`, render target sample-count validation, and backend-wgpu mapping to native multisample pipeline state.
- `headless_rhi_resolves_rgba8_msaa_texture_explicitly` and `wgpu_rhi_resolves_rgba8_msaa_texture_explicitly`: passed through `rhi_resolves`, covering explicit RGBA8 MSAA resolve validation, headless resolved payload semantics, and backend-wgpu native render-pass resolve.
- `headless_rhi_resolves_rgba8_msaa_texture_with_first_sample_mode`, `wgpu_rhi_resolves_rgba8_msaa_texture_with_first_sample_mode`, and `graph_pass_context_resolves_msaa_texture_with_first_sample_mode`: passed through `rhi_resolves` and `graph_pass_context_resolves`, covering mode-selectable `FirstSample` / `Sample(u32)` custom RGBA8 MSAA resolve through RHI and RenderGraph callback APIs, including non-zero sample-index selection and out-of-range sample-index validation.
- `headless_rhi_rejects_custom_resolve_shader`, `wgpu_rhi_resolves_rgba8_msaa_texture_with_custom_shader`, and `graph_pass_context_resolves_msaa_texture_with_custom_wgsl_shader`: passed through `rhi_resolves` and `graph_pass_context_resolves`, covering backend-wgpu user-supplied WGSL RGBA8 MSAA resolve shader execution, RenderGraph callback integration, and explicit headless unsupported behavior.
- `wgpu_rhi_resolves_rgba16f_msaa_texture_with_custom_shader`: passed through `rhi_resolves`, covering backend-wgpu user-supplied WGSL RGBA16F/HDR MSAA resolve shader execution and half-float output validation.
- `wgpu_rhi_resolves_rgba32f_msaa_texture_with_custom_shader`: passed through `rhi_resolves`; the test is gated on wgpu guaranteed-format MSAA support, so unsupported adapters validate the capability branch without triggering backend validation panics while supporting adapters execute the RGBA32F shader path.
- `wgpu_rhi_resolves_depth32f_msaa_texture_with_custom_shader` and `graph_pass_context_resolves_depth32f_msaa_texture_with_custom_wgsl_shader`: passed through `rhi_resolves` and `graph_pass_context_resolves`, covering backend-wgpu custom Depth32F MSAA resolve shaders that sample `texture_depth_multisampled_2d` and write a single-sample `frag_depth` target, plus RenderGraph callback integration and explicit headless unsupported behavior.
- `wgpu_rhi_resolves_rgba8_srgb_msaa_texture_with_custom_fragment_shader`, `wgpu_rhi_resolves_bgra8_srgb_msaa_texture_with_custom_fragment_shader`, and `graph_pass_context_resolves_srgb_msaa_texture_with_custom_fragment_shader`: passed through focused filters plus `rhi_resolves` / `graph_pass_context_resolves`, covering the current public sRGB/BGRA 8-bit custom resolve path through the fragment render-pass ABI.
- `backend_surface_readback_unsupported_marks_main_color_graph_export_unpromoted`: passed, covering unsupported backend surface readback provenance for exported standard-frame `main_color` graph resources and ensuring unpromoted surface exports are not misclassified as imported public graph writebacks.
- `surface_graph_export_support_reports_direct_swapchain_export_unsupported`: passed, covering public support-query reporting for unsupported direct swapchain graph export and readback-backed surface graph export support/enabled state.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_ -- --nocapture`: 13 passed, covering backend-wgpu transient export promotion, transient partial RGBA8/RGBA16F/RGBA32F/Depth32Float texture-region and buffer-range promotion, imported texture writeback, imported RGBA8/RGBA16F/RGBA32F/Depth32Float texture region writeback, layered and cross-layer RGBA8 flattened region writeback, represented non-base mip RGBA8 region writeback, generated mip-chain regeneration across D1/D2/D2Array/D3/Cube/CubeArray, imported full-buffer writeback, and imported partial/disjoint buffer export range writeback.
- `cargo test -p engine_renderer register_graph_extension_rejects_empty_names`: passed, including public graph extension registration rejecting empty/blank names.
- `cargo build -p render_facade_window_usecase`: passed after the windowed facade example was updated to request GPU profiling and expose GPU-time/profiler gate state in the window title.
- `cargo test -p engine_renderer frame_builds_stats_from_scene_and_view`: passed, including high-level `FrameStats::gpu_time_ms` populated from headless RHI timestamp results when GPU profiling is enabled, the graph has no imported facade resources, and pass-level `RenderGraphStats::rhi_executed_pass_labels` shows concrete standard 3D passes such as `gpu_culling`, `gpu_deformation`, `depth_prepass`, `gbuffer`, `ssao`, `deferred_lighting`, `motion_vectors`, `taa`, and `present` entered the RHI execution path.
- `cargo test -p engine_renderer renderer_config_controls_debug_label_groups`: passed, confirming the RHI profiling path preserves debug-label graph stats.
- `cargo test -p engine_renderer renderer_config_controls_transient_resource_aliasing_stats`: passed, confirming the RHI profiling path preserves transient aliasing graph stats.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer pipeline -- --nocapture`: 18 passed, including shader reload, shader destroy, material template destroy, backend-wgpu native pipeline replacement tombstones, `PipelineCacheBackendCoverage`, facade/backend-object gap reporting, aggregate per-frame cache usage, and public `PipelineCacheEntryInfo` / `PipelineCacheEntryStatus` entries plus per-entry `last_used_frame` / `used_this_frame` usage observability.
- `cargo test -p engine_renderer shader -- --nocapture`: 16 passed, including shader variant warmup/cache observability, feature subset validation, per-frame variant use reset, and shader reload/destroy invalidating variant cache entries.
- `cargo test -p render_wgpu mesh_renderer_static_pipeline_inventory_is_reported -- --nocapture`: passed, covering the public static native render pipeline inventory used by backend-wgpu frame stats.
- `cargo test -p engine_renderer material_creation_rejects_destroyed_template_dependencies`: passed, including destroyed shader rejection for material template creation and destroyed template rejection for material creation.
- `cargo test -p engine_renderer pipeline_warmup_validates_pipeline_keys`: passed, including DestroyQueued shader/template rejection, non-zero vertex layout hash validation, pipeline shader/material-template shader mismatch rejection, unsupported material render-phase rejection, pipeline sample-count / renderer MSAA mismatch rejection, and pipeline feature-bits subset / current-pass support validation for pipeline warmup.
- `cargo test -p engine_renderer renderer_config_rejects_invalid_latency_and_msaa`: passed, including rejection of MSAA sample counts that cannot be represented by `PipelineKey::sample_count`, rejection of unsupported surface color formats not advertised by default `FormatCaps`, and acceptance of every format in `RendererCaps::default().formats.color` for valid configs.
- `cargo test -p engine_renderer capture -- --nocapture`: 2 passed, including request-time external capture hook validation, explicit rejection of replacing an already queued capture request, public pending capture inspection through `Renderer::pending_frame_capture_info` with live backend status/integration/unavailable-reason data, execution-time `BackendUnavailable` status if a queued hook is removed before frame finish, public `FrameCaptureBackendInfo` availability/status queries before and after external hook registration, public `FrameCaptureIntegration` / `sdk_name` / `unavailable_reason` reporting for RenderDoc and external-debugger capture backends, `FrameCapture` preserving the capture request id, queued frame index, capture latency, and frame-finish backend integration / SDK / unavailable-reason snapshot, registered hook metadata through `FrameCaptureHookDesc`, `Renderer::register_frame_capture_backend_hook`, `Renderer::unregister_frame_capture_backend_hook`, `registered_hook_label`, and `registered_sdk_name`, `FrameCapture::external_hook_triggered` plus `external_hook_label` / `external_hook_sdk_name` reporting registered external-hook handoff metadata at frame finish, plus `FrameCapture::pipeline_cache` preserving per-frame pipeline cache stats in capture payloads.
- `cargo test -p engine_renderer frame_rejects_material_template_with_destroyed_shader`: passed, including frame-time rejection of a material template whose shader was destroyed after material creation.
- `cargo test -p engine_renderer render_graph_extensions`: 3 passed, including destroyed import, usage mismatch, and indirect buffer import validation.
- `cargo test -p engine_renderer render_graph_extensions_reject_destroyed_imported_renderer_resources`: passed, including destroyed texture and buffer handles imported by custom `RenderGraphExtension`.
- `cargo test -p engine_renderer render_graph_extensions_reject_imported_renderer_resource_usage_mismatches`: passed, including texture and buffer usage mismatches imported by custom `RenderGraphExtension`.
- `cargo test -p engine_renderer render_graph_extensions_validate_indirect_imported_buffer_usage`: passed, including accepted `BufferUsage::INDIRECT` and rejected non-indirect buffer imports.
- `cargo test -p engine_renderer renderer_public_flags_support_bitflag_queries`: passed, including `BufferUsage::INDIRECT` bitflag coverage.
- `cargo test -p engine_renderer scene_command_buffer_rejects_destroyed_resource_handles_before_mutation`: passed, including destroyed mesh/material/environment and LOD group dependency preflight.
- `cargo test -p engine_renderer scene_`: 13 passed, including ECS-like extract data driving scene command buffers and producing visible frame stats.
- `cargo test -p engine_renderer ecs_like_extract_fixture_drives_scene_commands_and_frame_stats -- --nocapture`: passed, covering multi-entity extract, light/environment scene commands, retained scene storage, and headless frame stats from the extracted scene.
- `cargo test -p engine_renderer render_targets_must_match_renderer_msaa_samples -- --nocapture`: passed, covering renderer MSAA sample-count validation for direct texture targets, texture-view targets, and external render target descriptors.
- `cargo test -p engine_renderer render_targets_must_use_formats_supported_by_caps -- --nocapture`: passed, covering renderer format-cap validation for direct texture targets, texture-view targets, external render target descriptors at creation and frame time, headless render targets, and depth attachments.
- `cargo test -p engine_renderer texture_view_render_targets_validate_subresource_ranges -- --nocapture`: passed after render target sample-count validation, covering texture-view mip/layer/dimension constraints.
- `cargo test -p engine_renderer render_targets_are_validated_and_can_back_offscreen_views -- --nocapture`: passed after render target sample-count validation, covering offscreen render target validation and rendering.
- `cargo test -p engine_renderer generate_mips -- --nocapture`: 6 passed, including retained RGBA8 2D/layered/volume mip generation, retained RGBA32F 2D and D3 volume chain generation, unsupported/missing data errors, and `TextureInfo::mips_generated` observability resetting after texture updates.
- `cargo test -p engine_renderer texture -- --nocapture`: 20 passed, including texture creation/update validation, render-target texture validation, graph/RHI texture usage, and mip-generation observability compatibility.
- `cargo test -p engine_renderer custom_material_parameters_are_schema_validated`: passed, including destroyed texture/sampler material update assertions.
- `cargo test -p engine_renderer standard_graph_import_rejects_destroyed_environment_textures`: passed, including destroyed environment texture validation for graph import and frame output texture labels.
- `cargo test -p engine_renderer environment_`: 3 passed, including environment IBL slot validation, environment graph import validation, profiler coverage for imported environment textures, environment frame outputs exposing IBL texture labels/mip counts/generated-mip state, and environment bake producing a complete retained prefiltered-specular mip chain marked through `TextureInfo::mips_generated`.
- `cargo test -p engine_renderer bindless_textures_require_capability_and_track_texture_table_pass`: passed, including destroyed material texture validation for bindless texture table graph construction.
- `cargo test -p engine_renderer bindless`: 2 passed.
- `cargo test -p engine_renderer virtual_texturing_requires_capability_and_tracks_feedback_pass`: passed, including destroyed material texture validation for virtual texture feedback graph construction and streaming output stats.
- `cargo test -p engine_renderer virtual_texturing`: passed.
- `cargo test -p engine_renderer resource_residency_controls_streamed_meshes_and_textures`: passed, including `MemoryStats::resident_resources` and `MemoryStats::evicted_resources` observability for evict/make-resident transitions.
- `cargo test -p engine_renderer lod_frame_output_rejects_destroyed_lod_level_resources`: passed.
- `cargo test -p engine_renderer lod`: 3 passed.
- `cargo test -p engine_renderer deformation`: 2 passed, including destroyed skeleton and morph resource validation for deformation stats plus frame deformation output reporting unique skeleton/morph resource counts and buffer-byte footprints.
- `cargo test -p engine_renderer motion_vector`: 3 passed, including destroyed mesh validation for motion vector stats and frame motion-vector output reporting moving mesh counts plus vertex-byte footprints.
- `cargo test -p engine_renderer frame_stats_report_resident_memory_and_delayed_destroy_count`: passed, including `MemoryStats::reclaim_policy`, `MemoryStats::delayed_destroy_bytes`, `MemoryStats::reclaimed_this_frame`, and `MemoryStats::reclaimed_bytes_this_frame` reporting frame-latency reclamation, delayed-memory pressure, and zero reclaim while resources remain delayed.
- `cargo test -p engine_renderer generic_resource_lifecycle_covers_public_resource_kinds`: passed, including `ResourceReclaimPolicy::FrameLatency`, `MemoryStats::delayed_destroy_bytes` remaining stable during frame-latency delay, then moving into `MemoryStats::reclaimed_bytes_this_frame` with the exact reclaimed count when resources become invalid.
- `cargo test -p engine_renderer wait_for_gpu_reclaims_destroy_queued_resources_without_frame_latency`: passed, including `FrameInput::wait_for_gpu` flushing pending uploads, switching frame memory observability to `ResourceReclaimPolicy::BackendFence`, and reclaiming DestroyQueued resources before configured frame latency elapses after backend/headless GPU-idle synchronization.
- `cargo test -p engine_renderer submitted_frame_ -- --nocapture`: passed, covering both default submitted-frame upload/staging completion and empty default frames preserving DestroyQueued resources while submitted frames reclaim them through `ResourceReclaimPolicy::SubmissionBoundary` when the submission boundary is complete.
- `cargo test -p engine_renderer poll_resource_retirements_completes_only_prior_submission_work -- --nocapture`: passed, covering explicit non-blocking retirement polling, per-submission upload batch accounting, and future-frame DestroyQueued resources staying delayed until covered by a later completed submission boundary.
- `cargo test -p engine_renderer frame_capture_resource_dump_counts_only_ready_resources`: passed, including capture resource dumps mirroring `MemoryStats::resident_resources`, `MemoryStats::evicted_resources`, `MemoryStats::reclaim_policy`, `MemoryStats::delayed_destroy_bytes`, `MemoryStats::reclaimed_this_frame`, `MemoryStats::reclaimed_bytes_this_frame`, and counting ready renderer-generated mip textures through `FrameCaptureResourceDump::generated_mip_textures`.
- `cargo test -p engine_renderer frame_capture_resource_dump_excludes_destroyed_inactive_resources_when_active_resources_remain`: passed, including inactive destroyed mesh/texture handles excluded from capture resource dump counts while active scene resources remain visible.
- `cargo test -p engine_renderer frame_capture_resource_dump_counts_zero_when_only_destroyed_resources_in_capture_frame`: passed, including capture resource dump counting ready-only semantics when no active resources remain and only DestroyQueued resources are in-scope (both mesh and texture counts zero, with delayed/reclaimed totals preserved).
- `cargo test -p engine_renderer wgpu_metrics_`: 2 passed, including wgpu backend frame stats carrying an explicit `ResourceReclaimPolicy` and native `rhi_executed_pass_labels`.
- `cargo test -p engine_renderer frame_wait_for_gpu_flushes_pending_upload_stats`: passed, including `UploadStats::bytes_queued_this_frame`, `UploadStats::uploads_queued_this_frame`, `UploadStats::staging_bytes_queued_this_frame`, `UploadStats::bytes_uploaded_this_frame`, `UploadStats::uploads_completed_this_frame`, and `UploadStats::staging_bytes_released_this_frame` resetting on the next frame so frame-local upload data does not leak across frames.
- `cargo test -p engine_renderer upload_stats_track_pending_staging_until_flush`: passed, including manual `flush_uploads` preserving immediate queued-this-frame, uploaded-this-frame, completed-upload-count, staging-queued, and released-staging-byte observability outside frame boundaries.
- `cargo test -p engine_renderer capture`: 2 passed.
- `cargo build -p render_facade_window_usecase`: passed after the windowed facade example was converted to a repeatable smoke target with `--smoke-frames`, `--wait-for-gpu`, `--print-stats`, and `--require-gpu-time` options.
- `.\target\debug\render_facade_window_usecase.exe --smoke-frames 3 --wait-for-gpu --print-stats`: passed on the local visible-window/surface path, printing `surface-smoke frame=2 draws=1 visible=1 profiler=true gpu_time_ms=Some(0.26964) graph_passes=21 rhi_passes=21 semantic_passes=0`.
- `cargo build -p render_scene_usecase -p render_facade_usecase -p render_facade_window_usecase -p render_smoke -p render_feature_showcase`: passed.
- Wgpu-backed graph/RHI tests are serialized with a test-only guard to avoid Windows teardown crashes from concurrent device creation/destruction.

| Area | Status | Current evidence | Remaining work |
| --- | --- | --- | --- |
| Renderer facade config/init/caps | Implemented | `Renderer::new`, `Renderer::with_surface`, `Renderer::new_headless`, `RendererCaps`, backend feature gates exist in `Render/engine_renderer/src/lib.rs`; wgpu runtime exists in `backend_wgpu.rs`. Vulkan/Metal/D3D12 preferences now return explicit `UnsupportedFeature` instead of silently using wgpu; covered by `renderer_new_selects_configured_backend_without_surface` and `surface_backend_preference_accepts_only_surface_backends`. MSAA config is constrained to values representable by `PipelineKey::sample_count`; covered by `renderer_config_rejects_invalid_latency_and_msaa`. `RendererFeatures::SURFACE` now exists and wgpu caps only advertise it when the runtime owns a surface, so `Renderer::new(BackendPreference::Wgpu)` no longer claims `RendererFeature::Surface` without `Renderer::with_surface`. `Renderer::feature_info` exposes per-feature support, stability tier, and unsupported reason. `validate_surface_backend_preference` now reports `UnsupportedFeature(RendererFeature::BackendWgpu)` for `BackendPreference::Auto`/`Wgpu` when the backend-wgpu feature is unavailable, and `with_surface` now surfaces that directly; covered by `surface_backend_preference_accepts_only_surface_backends` and `with_surface_requires_backend_wgpu_if_unavailable`. `validate_renderer_config` now validates `surface_format` against `RendererCaps::formats.color` and accepts the entire default advertised format set; covered by `renderer_config_rejects_invalid_latency_and_msaa`. `WgpuRendererRuntime::with_surface` now validates requested `surface_format`/`depth_format` against the created surface and runtime caps; covered by `validate_surface_runtime_formats_rejects_configured_color_format_mismatch` and `validate_surface_runtime_formats_rejects_configured_depth_format_mismatch` in `backend_wgpu.rs`. | `with_surface` now preserves runtime-formats consistency for surface creation, rather than accepting requested caps-only values that later diverge from actual runtime behavior. |
| Surface lifecycle | Implemented | Main surface handle, resize, vsync, and wgpu surface path exist. `main_surface_handle_participates_in_resource_queries` covers main surface status/priority queries, stale surface status/priority, priority updates, explicit destroy rejection for renderer-owned main surface, and invalid-handle errors for stale surface destroy. `render_targets_are_validated_and_can_back_offscreen_views` covers invalid `RenderTarget::Surface` handles and resized main-surface extent propagation. | Keep visible-window launch in final example verification, but facade surface lifecycle semantics are covered. |
| Type-safe handles | Implemented | Typed `Handle<T>` and resource tags exist. | Stale/generation mismatch semantics are now covered across destroy/info/update/query paths: `generic_resource_generation_mismatch_returns_invalid_handle_for_stale_handles`, `buffer_update_rejects_stale_generation_handles`, `mesh_update_rejects_stale_generation_handles`, `texture_update_and_generate_mips_reject_stale_generation_handles`, and `info_queries_return_none_for_stale_generation_handles` keep invalid-handle behavior stable per handle kind. |
| Resource status/destroy | Partial | Generic `resource_status` and `destroy` exist; delayed destroy count is reported. Public renderer resources now remain `DestroyQueued` until `frame_latency` elapses, then old handles become invalid; covered by `generic_resource_lifecycle_covers_public_resource_kinds`. `ResourceReclaimPolicy` exposes the normal frame-latency strategy, `MemoryStats::delayed_destroy_bytes` reports queued delayed-destroy byte pressure, `MemoryStats::reclaimed_this_frame` reports the exact number of resources reclaimed on the current frame, `MemoryStats::reclaimed_bytes_this_frame` reports reclaimed resource bytes, and capture resource dumps mirror those values. `FrameInput::wait_for_gpu` now waits/polls the backend when present, flushes pending uploads, exposes `ResourceReclaimPolicy::BackendFence` in the frame memory stats, and reclaims DestroyQueued resources without waiting for configured frame latency; covered by `wait_for_gpu_reclaims_destroy_queued_resources_without_frame_latency`. Default submitted frames now reclaim DestroyQueued facade resources at a completed submission boundary and report `ResourceReclaimPolicy::SubmissionBoundary`, while empty frames preserve delayed resources; covered by `submitted_frame_reclaims_destroy_queued_resources_at_completed_submission_boundary`. `Renderer::poll_resource_retirements` exposes the same non-blocking completed-boundary retirement path outside frame finish and keeps future-frame DestroyQueued resources delayed until a later completed submission boundary covers them; covered by `poll_resource_retirements_completes_only_prior_submission_work`. Cooperative background retirement startup/observability is supported through `Renderer::start_background_resource_retirement`, `stop_background_resource_retirement`, `background_resource_retirement_active`, `ResourceRetirementStats::background_retirement_active`, and `MemoryStats::background_retirement_active`; covered by `background_resource_retirement_can_be_started_and_observed`. `ResourceLifecycleSupport` / `Renderer::resource_lifecycle_support()` reports lifecycle/stale-handle/upload-readback/residency/debug-capture coverage per resource class, and the same support matrix now propagates through `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`; covered by `resource_lifecycle_support_reports_per_class_lifecycle_and_backend_gaps` and `frame_debug_report_summarizes_last_frame_for_editor` propagation assertions. `BackendSynchronizationSupport` / `Renderer::backend_synchronization_support()` reports submission-boundary retirement, backend tombstone retirement, queue-empty fallback polling, true nonblocking submission-index polling, and background scheduler active state, and the same support matrix now propagates through `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`; covered by `backend_synchronization_support_reports_polling_and_scheduler_limits` and frame-debug propagation assertions. Wgpu surface/direct submissions now record the latest `wgpu::SubmissionIndex`, and backend wait prefers `WaitForSubmissionIndex` before falling back to device-wide wait; backend default frames also poll submissions non-blockingly before using the submission-boundary reclaim path. | Per-resource backend fence objects remain partial: completion is still tombstone-order driven with completion-index polling and queue-empty fallback, and backend lifetime objects are not yet represented as independent persistent per-resource fences. |
| Mesh / buffer API | Implemented | Create/update/info paths exist for meshes and buffers. Public `BufferUsage` now includes `INDIRECT`, covered by `renderer_public_flags_support_bitflag_queries`. | Update-range validation and stale-generation update-handle behavior are now exercised by `buffer_resources_validate_size_usage_and_update_ranges`, `buffer_update_rejects_stale_generation_handles`, and `mesh_update_rejects_stale_generation_handles`. |
| Texture / sampler API | Partial | Texture create/update/info, sampler creation, mip generation, sampler info, and bytes inspection exist. `TextureConfigurationStats` / `Renderer::texture_configuration_stats()` now aggregate Ready sampled, render-target, copy-src, multi-mip, generated-mip, and multisampled texture usage, and propagate through `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`. `SamplerInfo` / `Renderer::sampler_info()` expose sampler descriptor/status inspection while `resource_status` remains the destroyed/queued-destroy status surface; covered by `sampler_info_reports_desc_status_and_destroyed_payload_boundary`. `SamplerConfigurationStats` / `Renderer::sampler_configuration_stats()` now aggregate Ready compare, anisotropic, and custom-LOD sampler usage, and the same summary is propagated through `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`; the dump also preserves flat `comparison_samplers`, `anisotropic_samplers`, and `custom_lod_samplers` fields for capture consumers. This lets frame/capture/debug artifacts observe texture/sampler feature usage instead of only total resource counts; covered by `frame_capture_resource_dump_counts_only_ready_resources` and `frame_debug_report_summarizes_last_frame_for_editor` propagation assertions. `TextureInfo::mips_generated` now makes renderer-generated mip chains observable to tools, and `FrameCaptureResourceDump::generated_mip_textures` carries that observability into capture payloads. Retained RGBA8 2D/layered/volume mip generation, retained RGBA32F 2D and D3 volume chain generation, unsupported/missing data errors, update-after-generation invalidation, and capture resource-dump counting are covered by `generate_mips`, including `generate_mips_builds_retained_rgba32f_chain` and `generate_mips_builds_retained_rgba32f_volume_chain`, and `frame_capture_resource_dump_counts_only_ready_resources`, while broader texture validation is covered by `texture`. `texture_mutation_rejects_destroyed_handle` now covers mutation APIs after destroy, and `texture_queries_return_none_for_destroyed_handle` + `info_queries_return_none_for_stale_generation_handles` covers stale/missing-texture-query behavior. Texture-view and plain texture frame outputs now reject destroyed texture targets through `texture_view_frame_output_rejects_destroyed_target_texture` and `render_view_rejects_destroyed_texture_target`, while `build_view_graph_stats` now returns `InvalidHandle` for destroyed plain texture targets via `build_view_graph_stats_rejects_destroyed_texture_target` and for destroyed texture views via `build_view_graph_stats_rejects_destroyed_texture_view_target`. Backend-wgpu facade texture upload can now express base-level upload plus native GPU mip generation for D2/D2Array/Cube/CubeArray sampled material textures (including RGBA16F and RGBA32F) instead of per-mip CPU upload, with `WgpuMaterialTextureBinding::generated_mips` observability; covered by `wgpu_reflected_facade_texture_upload_uses_gpu_mip_generation_for_2d_rgba8`, `wgpu_material_texture_binding_generates_mips_on_gpu`, `wgpu_material_texture_binding_generates_float_mips_on_gpu`, `wgpu_material_array_texture_binding_generates_layer_mips_on_gpu`, `wgpu_material_cube_texture_binding_generates_face_mips_on_gpu`, and `wgpu_material_texture_gpu_mip_generation_rejects_invalid_descs`. Streamable texture mips are now observable in `MemoryStats`/capture resource dumps with resident-vs-evicted splits. | Public `Renderer::generate_mips` still retains CPU-generated mip bytes for headless/tooling compatibility, while backend GPU mip generation now supports filterable 8-bit and float sampled material textures for 2D/layered/volume paths. |
| Shader API | Partial | Shader create, reflection descriptors, reload from file/desc, compatibility validation, hot reload tests, and public renderer-layer shader variant cache APIs exist. `Renderer::warm_up_shader_variants`, `Renderer::shader_variant_info`, and `Renderer::shader_variant_cache_entries` expose variant cache entries with canonicalized feature flags, shader interface layout hash, backend-compiled status, last-used frame, and used-this-frame state. `ShaderVariantCacheStats` / `Renderer::shader_variant_cache_stats()` expose aggregate entries, used-this-frame, ready-unused, backend-compiled, missing-backend-module, and interface-layout counts from the same source of truth used by `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`. Variant warmup rejects undeclared shader features, compiles/caches backend-wgpu `wgpu::ShaderModule` entries when a wgpu runtime exists, exposes aggregate cache pressure through frame/debug/capture/resource-dump stats, and shader reload/destroy invalidates matching renderer/backend variant entries, with invalidated backend-wgpu variant `wgpu::ShaderModule` entries moving into backend-owned tombstones and explicit poll retirement; covered by `shader_variant_cache_tracks_features_and_invalidates_with_shader`, `shader_variant_warmup_compiles_backend_shader_module_when_wgpu_runtime_exists`, `frame_debug_report_summarizes_last_frame_for_editor`, and `wgpu_shader_variant_module_cache_compiles_reuses_and_invalidates`. Pipeline warmup and frame pipeline-key generation now require shader handles to be Ready, including DestroyQueued shader rejection covered by `pipeline_warmup_validates_pipeline_keys` and `frame_rejects_material_template_with_destroyed_shader`. Shader reload/destroy now also calls backend-wgpu native reflected pipeline invalidation for matching shader keys; `wgpu_native_cache_reuses_render_pipeline_across_material_bind_groups` covers batch native invalidation and render pipeline object cleanup. | Backend shader modules for warmed variants are now cached and invalidated; broader variant-to-native-pipeline/material-template permutation integration remains future backend work. |
| Material API | Partial | Standard material, custom material, material templates, parameter schema, fast parameter updates exist. Material create/update paths validate texture and sampler resource handles, including destroyed texture/sampler handles covered by `custom_material_parameters_are_schema_validated`. Material template creation rejects DestroyQueued shader handles, material creation rejects DestroyQueued template handles, and material template shader dependencies are revalidated during pipeline warmup and frame draw-item generation. Material template destroy and material parameter updates now also call backend-wgpu native reflected pipeline invalidation for matching template/material keys. `MaterialBackendSupport` / `Renderer::material_backend_support()` now expose facade material support, shader-reflection/schema diagnostics, backend-wgpu reflected custom-material draw support, backend texture/sampler binding support, and the still-unsupported complete dynamic material-template backend pipeline boundary. The same support matrix now propagates through `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`, so tools can observe the dynamic-template backend gap from frame/capture artifacts without issuing a separate query. `MaterialReflectionCoverageStats` / `Renderer::material_reflection_coverage_stats()` now aggregate Ready material/template counts, pipeline-ready counts, shader-interface/template readiness, schema/material reflection coverage, incomplete coverage counts, and missing reflected texture/sampler/buffer bindings, with the same summary propagated through `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`; covered by `material_template_schema_is_validated_against_shader_reflection`, `frame_debug_report_summarizes_last_frame_for_editor`, and `material_backend_support_distinguishes_facade_reflected_backend_and_dynamic_template_gap`, alongside `wgpu_native_cache_reuses_render_pipeline_across_material_bind_groups`, `material`, and `pipeline`. | Need broader backend/material-template integration beyond the reflected wgpu custom-material subset, especially complete dynamic material-template pipeline-layout/bind-group integration. |
| Scene retained mode | Implemented | Scene creation, scene editor, command buffer, object/light/environment updates exist. `SceneCommandBuffer` now prevalidates command resource handles before mutating the retained scene, including destroyed mesh/material/environment handles and LOD group internal mesh/material dependencies; covered by `scene_command_buffer_rejects_destroyed_resource_handles_before_mutation`. ECS-like extract data now drives multi-object, light, and environment scene commands into retained scene storage and headless frame stats; covered by `ecs_like_extract_fixture_drives_scene_commands_and_frame_stats`. | Continue broad stale-handle tests as other systems integrate. |
| Camera / view / render target | Implemented | Camera, render target, view descriptors, viewport/scissor/quality settings exist. Direct texture targets, texture-view targets, external render target descriptors, and headless render targets now validate color/depth formats against `RendererCaps::formats` and validate sample counts against `RendererConfig::msaa_samples`; external render targets are revalidated at frame time so caps changes and destroyed color/depth attachments cannot bypass validation. Texture-view targets also validate mip/layer ranges and 2D-compatible dimensions; offscreen render target validation/rendering remains covered. Covered by `render_targets_must_match_renderer_msaa_samples`, `render_targets_must_use_formats_supported_by_caps`, `texture_view_render_targets_validate_subresource_ranges`, `render_targets_are_validated_and_can_back_offscreen_views`, `external_render_target_rejects_destroyed_attachment_at_frame_time`, and `external_render_target_rejects_destroyed_depth_attachment_at_frame_time`. | Continue checking new backend-specific format constraints as backend caps evolve. |
| Light / shadow / environment / IBL | Partial | Directional/point/spot/area lights, shadows, environment descriptors, environment outputs and legacy fallback exist. Standard graph environment texture import and environment frame output texture labels now validate sampled/ready texture handles and reject destroyed environment textures; covered by `standard_graph_import_rejects_destroyed_environment_textures`. Environment frame outputs now expose skybox/irradiance/prefiltered-specular/BRDF texture labels, mip counts, and generated-mip state so baked IBL resources are observable through frame stats/debug/capture. Environment bake now creates ready irradiance/prefiltered-specular/BRDF resources, writes a complete retained prefiltered-specular mip chain for the requested mip count, and marks that texture through `TextureInfo::mips_generated`; covered by `environment_`. `RendererLightingSupport` / `Renderer::lighting_support()` distinguishes retained lights, graph-observable shadows, retained environment IBL, backend IBL convolution, and runtime environment capture. The same support matrix now propagates through `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`, so editor/capture artifacts expose backend IBL convolution and runtime capture gaps without issuing a separate query; covered by `lighting_support_distinguishes_retained_lighting_from_backend_ibl_convolution` and `frame_debug_report_summarizes_last_frame_for_editor` propagation assertions. | IBL/environment bake remains renderer-retained/facade generated rather than full backend-real convolution/capture for every path. |
| Animation / skinning / morph / LOD | Partial | Skeleton instances, morph weights, LOD groups, deformation, LOD, and motion-vector frame outputs exist. LOD group dependencies are validated during frame scene preflight, scene command assignment, and `FrameLodOutput` construction, including destroyed level mesh/material resources. Deformation stats reject destroyed skeleton/morph resources before reporting skinned or morphed objects, and `FrameDeformationOutput` now reports unique skeleton/morph resource counts plus skeleton/morph buffer-byte footprints alongside the deformed vertex output buffer. Motion-vector stats reject destroyed selected meshes before reporting moving objects, and `FrameMotionVectorOutput` reports moving mesh counts plus vertex-byte footprints for the motion-vector pass while preserving camera-only/TAA/motion-blur outputs. `DeformationSupport` / `Renderer::deformation_support()` distinguishes facade-retained skeletal/morph state, graph-observable LOD/motion-vector outputs, and backend GPU deformation support. The same support matrix now propagates through `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`, so editor/capture artifacts expose the backend GPU deformation gap without issuing a separate query; covered by `deformation_support_distinguishes_facade_outputs_from_backend_gpu_path` and `frame_debug_report_summarizes_last_frame_for_editor` propagation assertions. | Need deeper backend execution path for GPU skinning/morph/motion-vector shader buffers beyond current renderer/RHI observability. |
| Frame API / stats / capture | Implemented | `begin_frame`, `Frame::render_view`, `finish`, `FrameStats`, `FrameCapture`, capture queues exist. External capture requests now require a registered backend hook before queuing, a second capture request is rejected while one is already pending so queued requests cannot be silently replaced, and `Renderer::pending_frame_capture_info` exposes the pending request plus current backend status/integration/unavailable-reason data before frame finish; covered by `capture_options_validate_backend_hooks`. `FrameCaptureHookDesc`, `Renderer::register_frame_capture_backend_hook`, and `Renderer::unregister_frame_capture_backend_hook` expose explicit external-hook registration metadata while the compatibility `set_frame_capture_backend_available` path remains covered. `FrameCaptureBackend::all`, `FrameCaptureBackendInfo`, `FrameCaptureIntegration`, `Renderer::frame_capture_backend_info`, and `Renderer::frame_capture_backend_infos` expose capture backend availability, hook requirements, SDK/dependency names, registered hook labels, registered SDK names, unavailable reasons, and request status for tools. `FrameCapture` now snapshots capture request id, queued frame index, capture latency, backend integration, external-hook requirement, SDK name, and unavailable reason at frame finish, so capture payloads remain self-describing after backend registration changes. `FrameCapture::external_hook_triggered` / `external_hook_label` / `external_hook_sdk_name` and matching `FrameDebugReport` fields expose whether a registered external-hook handoff survived through frame finish and which hook was handed off. `FrameCapture::pipeline_cache` preserves per-frame pipeline cache stats in capture payloads. Frame capture resource dumps count only Ready resources, report generated mip texture counts through `generated_mip_textures`, report reclamation policy through `reclaim_policy`, DestroyQueued resources through `delayed_destroy_count` / `delayed_destroy_bytes`, and current-frame reclamation through `reclaimed_this_frame` / `reclaimed_bytes_this_frame`; covered by `frame_capture_resource_dump_counts_only_ready_resources`, `frame_capture_resource_dump_excludes_destroyed_inactive_resources_when_active_resources_remain`, and `frame_capture_resource_dump_counts_zero_when_only_destroyed_resources_in_capture_frame`. | Native RenderDoc/external-debugger SDK invocation is tracked separately as `External Blocked`; the implemented frame API path is internal capture plus registered external-hook handoff with user-visible metadata and unavailable reasons. |
| RenderGraph resources/builder/pass/context | Implemented | Graph textures/buffers, pass builder, dependencies, barriers, transient aliasing, RHI execution hooks exist. Imported renderer texture/buffer resources are exposed for facade validation before graph execution. `RenderGraphStats::semantic_passes`, `RenderGraphStats::rhi_executed_passes`, and `RenderGraphStats::rhi_executed_pass_labels` now distinguish facade/graph-semantic passes from RHI/backend-executed passes and preserve pass-level RHI evidence, covered by `graph_` and `frame_builds_stats_from_scene_and_view`. | Continue to audit unsafe encoder lifetime workaround before final completion. |
| RenderGraph extension/custom pass | Implemented | `RenderGraphExtension`, `RenderPassNode`, post-process registration, outline pass, custom pass tests exist. Public graph extension registration rejects empty names. `examples/render_facade_usecase` includes a user-facing `CountingPass` custom graph extension and builds with `cargo build -p render_facade_usecase`. Custom graph extensions cannot import destroyed renderer texture/buffer handles or import resources with undeclared renderer usage. Indirect graph buffer imports require public `BufferUsage::INDIRECT`; covered by `register_graph_extension_rejects_empty_names`, `render_graph_extensions_reject_destroyed_imported_renderer_resources`, `render_graph_extensions_reject_imported_renderer_resource_usage_mismatches`, and `render_graph_extensions_validate_indirect_imported_buffer_usage`. | Keep example building as part of final verification. |
| Standard 3D graph | Partial | Deferred/forward graph labels, depth, shadow, gbuffer, light clustering, TAA, bloom, tonemap, FXAA and related tests exist. Environment graph imports, bindless texture table construction, and virtual texture feedback detection now validate renderer texture readiness before importing or enumerating external texture handles. `RenderGraphStats::semantic_passes` now makes graph-semantic standard/advanced passes observable instead of conflating them with backend-real execution. `RenderGraphStats::rhi_executed_pass_labels` now exposes concrete pass labels that entered RHI execution, and `frame_builds_stats_from_scene_and_view` covers standard 3D passes including `gpu_culling`, `gpu_deformation`, `depth_prepass`, `gbuffer`, `ssao`, `deferred_lighting`, `motion_vectors`, `taa`, and `present` on the profiled Headless RHI path. Backend-wgpu native frame stats now preserve actual wgpu `RenderPassDescriptor` labels for directional shadow cascades, spot shadows, point shadow cube faces, and the split forward opaque / transparent surface passes; skybox is observable as work inside `Neo Forward Opaque Pass`, not as a fake separate pass. Wgpu facade frames now merge facade semantic graph labels/barriers with backend native RHI labels/timestamps/GPU time instead of replacing one with the other. `PostProcessSupport` / `Renderer::post_process_support()` reports per-effect backend visibility, sampled-minimal implementation level, backend label tokens, and production-readiness gaps for HDR, bloom, TAA, FXAA, SSAO, SSR, depth of field, motion blur, tonemap, and color grading; the same support matrix now propagates through `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`; covered by `post_process_support_distinguishes_backend_visible_from_production_ready` and `frame_debug_report_summarizes_last_frame_for_editor` propagation assertions. | Some passes remain renderer-layer graph/RHI command semantics rather than full backend shader implementations. Production-grade post-process resource chains remain incomplete; backend-visible sampled branches are intentionally not marked production-ready. Keep converting individual standard passes to backend shader/material implementations where practical. |
| RHI abstraction | Partial | RHI device/encoder/resource/pipeline/pass/query traits and headless/wgpu devices exist. `RenderGraphBuilder::execute_on_rhi` and native wgpu frame metrics now report `RenderGraphStats::rhi_executed_passes`, while facade-semantic graph execution reports `semantic_passes`; `RenderGraphStats::rhi_executed_pass_labels` preserves pass-level RHI execution evidence for both graph/RHI execution and backend-wgpu native mesh rendering; wgpu facade frames merge both views into one `FrameStats::graph` snapshot. Covered by `graph_`, `frame_builds_stats_from_scene_and_view`, `wgpu_metrics_`, `default_wgpu_pass_labels_match_native_render_pass_order`, and `facade_backend_graph_merge_preserves_semantic_and_native_execution_stats`. | RHI is still mostly renderer-internal with partial backend execution coverage. |
| Pipeline / pipeline key / cache | Partial | Pipeline key, cache stats, warmup requests, pass flags, and tests exist. Pipeline warmup rejects DestroyQueued shader and material template handles, rejects zero vertex layout hashes, rejects pipeline keys whose shader does not match the material template shader, rejects render phases not supported by the material template pass flags, rejects sample counts that differ from renderer MSAA, and requires feature bits to be a non-empty subset of material template pass flags that supports the key render phase. Shader reload, shader destroy, and material template destroy invalidate dependent warmed pipeline cache entries, with `PipelineCacheStats::invalidated_this_frame` exposing the number of invalidated entries this frame. Backend-wgpu native reflected pipeline cache now has per-key creation, structural render-pipeline object reuse, material bind-group entry separation, and batch invalidation by shader/material template; `wgpu_native_cache_reuses_render_pipeline_across_material_bind_groups` covers reuse plus shader/template invalidation cleanup. Invalidated native reflected pipeline entries now move into backend-owned tombstones that keep shader modules, layout objects, material bind groups, owned buffers, and render-pipeline references alive until explicit completed-boundary poll retirement; the same test covers `WgpuBackendResourceRetirementStats`. `PipelineCacheStats::backend_objects` exposes unique native backend render pipeline object counts when supplied by backend-wgpu. Wgpu facade frames now merge renderer facade cache hit/miss/entry/invalidation counts with backend-wgpu native pipeline inventory instead of losing one side of the stats. `PipelineCacheStats::entries_used_this_frame` and `PipelineCacheStats::ready_unused_entries` expose aggregate frame usage; `ready_entries_without_backend_object` and `used_entries_without_backend_object` expose the current facade/backend-object gap instead of hiding it behind `backend_objects`. `Renderer::pipeline_cache_entries` exposes per-entry `PipelineCacheEntryInfo` / `PipelineCacheEntryStatus`; current facade-created entries report `has_backend_object: false` instead of pretending backend pipeline objects exist. Entries also report `last_used_frame` and `used_this_frame`, so tools can distinguish warmed-but-unused entries from pipeline keys actually consumed by the current frame. `FrameStats::pipeline_cache`, `FrameDebugReport::pipeline_cache`, and `FrameCapture::pipeline_cache` expose per-frame cache hit/miss/invalidation/backend-object/usage/gap data to frame tools. `render_wgpu::MeshRenderer` exposes its static native render pipeline inventory, and backend-wgpu copies that count into frame stats. Frame draw-item generation rejects material templates whose shader has been destroyed before recording pipeline keys. | Remaining pipeline work is broader standard-material/backend integration and future variant cache breadth, not reflected facade native object creation/invalidation. |
| GPU memory/upload/streaming | Partial | Upload stats, flush, priority, eviction/residency controls, memory stats exist. `MemoryStats::resident_resources` and `MemoryStats::evicted_resources` now make evict/make-resident transitions directly observable, and frame capture resource dumps mirror those counts. `MemoryStats` and `FrameCaptureResourceDump` now also expose streamable resource totals, resident/evicted streamable resource counts, streamable texture mip totals split by resident/evicted state, and streamable mesh byte totals split by resident/evicted state; covered by `resource_residency_controls_streamed_meshes_and_textures` and capture dump mirroring in `frame_builds_stats_from_scene_and_view`. Public resource delayed destroy now has explicit frame-latency reclamation reporting through `ResourceReclaimPolicy`, with `MemoryStats::delayed_destroy_bytes` exposing queued delayed-destroy byte pressure and `MemoryStats::reclaimed_this_frame` / `MemoryStats::reclaimed_bytes_this_frame` exposing the number and bytes of resources actually reclaimed on the current frame. `FrameInput::wait_for_gpu` now closes the explicit backend fence path by waiting for the latest recorded wgpu `SubmissionIndex` when present, flushing pending upload staging, marking frame memory stats as `ResourceReclaimPolicy::BackendFence`, and reclaiming delayed resources immediately. Default submitted frames now complete pending upload/staging stats when actual view/graph work was submitted and the submission boundary is complete, while empty frames keep uploads pending; covered by `submitted_frame_completes_pending_upload_stats_without_gpu_idle_wait`. Default submitted frames also reclaim DestroyQueued facade resources at a completed submission boundary and report `ResourceReclaimPolicy::SubmissionBoundary`; backend-wgpu checks this with non-blocking `Maintain::Poll`, while headless completed submissions are synchronous. `Renderer::poll_resource_retirements` exposes explicit non-blocking background polling for completed-boundary upload and destroy retirement; submitted uploads are now internally batched by submission frame so uploads queued after a submitted boundary remain pending until a later submission covers them. Cooperative background retirement startup is supported through `Renderer::start_background_resource_retirement()`, a lightweight scheduler thread, `Renderer::stop_background_resource_retirement()`, and public active-state observability in `MemoryStats` / `ResourceRetirementStats`; covered by `background_resource_retirement_can_be_started_and_observed`. Backend-wgpu native reflected pipeline invalidation, replacement, shader variant module invalidation, and material external texture/sampler unregister now create backend-owned tombstones; `Renderer::poll_resource_retirements` drives backend tombstone retirement, and `MemoryStats` / `FrameCaptureResourceDump` expose renderer-level `BackendResourceRetirementStats` for live and last-poll retired backend objects. `FrameStats`, `FrameDebugReport`, and `FrameCapture` now mirror `retired_submission_frame` and `pending_submission_frame` so tools and capture artifacts can observe submission-boundary retirement progress. `UploadStats::bytes_queued_this_frame`, `UploadStats::uploads_queued_this_frame`, `UploadStats::staging_bytes_queued_this_frame`, `UploadStats::bytes_uploaded_this_frame`, `UploadStats::uploads_completed_this_frame`, and `UploadStats::staging_bytes_released_this_frame` reset at frame start so per-frame upload stats do not leak across frames while manual `flush_uploads` remains observable outside frame boundaries. `FrameStreamingOutput` now requires streamable meshes and material textures to be Ready before reporting streaming stats, including destroyed material texture coverage through `virtual_texturing_requires_capability_and_tracks_feedback_pass`. | Upload/destroy no longer require explicit GPU idle for submitted-frame bookkeeping, explicit polling, completed-boundary facade resource release, cooperative background startup, and reflected native pipeline tombstone retirement, but true backend fence objects/nonblocking per-submission completion queries and tombstones for any remaining backend-owned resource classes remain incomplete. |
| ECS extract boundary | Implemented | `ExtractRenderData` and `SceneCommandBuffer` exist without binding renderer to a specific ECS. `ecs_like_extract_fixture_drives_scene_commands_and_frame_stats` provides an integration-style fixture with ECS-like renderables, light/environment commands, retained scene assertions, and a headless frame verifying extracted visible objects and draw calls. | Keep this fixture in final verification; add a standalone example only if final API docs require one. |
| Debug draw/editor API | Implemented | Debug draw commands, picking requests/tickets/results, frame outputs, legacy scene conversion exist. `DebugDraw::editor_gizmo`, `scene_object_gizmo`, `translation_gizmo`, and `scale_gizmo` now expose renderer-side editor gizmo overlay commands through the public facade. `scene_object_gizmo` validates scene/object handles, derives the gizmo transform from retained scene state, and marks the command as object-associated for editor picking/selection metadata. `FrameDebugDrawOutput` reports total command count plus primitive/text/editor-gizmo split counts, `pickable_editor_gizmo_count`, and exact `FrameEditorGizmoOutput { scene, object, kind }` records through `pickable_editor_gizmos`, so editor/debug consumers can distinguish retained overlay primitives from gizmo handles and map object-associated gizmos back to scene objects without parsing command internals. The backend-wgpu legacy scene conversion path expands translate/rotate/scale gizmos into native debug line meshes, while frame stats/debug reports observe all three gizmo command types through the standard debug overlay path; covered by the updated `scene_command_buffer_debug_draw_and_picking_are_exposed`, `frame_debug_report_summarizes_last_frame_for_editor`, and `debug_draw_lines_are_added_to_legacy_render_scene` assertions. `DebugToolingSupport` now describes debug draw plus editor-gizmo visualization as renderer-facade supported while keeping host editor event routing outside the renderer API. The same `DebugToolingSupport` support matrix now propagates through `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`, so debug/capture artifacts preserve debug draw, picking, frame report, frame capture, and native debugger SDK blocker state; covered by `debug_tooling_support_keeps_native_debugger_sdk_blocker_explicit` and `frame_debug_report_summarizes_last_frame_for_editor` propagation assertions. `FrameDebugReport` exposes an editor-facing summary of the last frame, including full `RenderGraphStats`, graph pass labels/counts/barriers, semantic-vs-RHI graph pass counts, pass-level `rhi_executed_pass_labels`, draw/dispatch/visibility counts, profiler state/GPU time, pipeline/material switches, pipeline cache hit/miss/invalidation/backend-object stats, upload/memory stats, submission-boundary retirement frame fields, culling/SSAO/light-cluster/area-light/ray-tracing/shadow/gbuffer/LOD/streaming/environment/deformation/motion-vector/post-process outputs, debug draw outputs, picking outputs, capture trigger/request parameters, capture label/backend/status/resource dump data, and pipeline statistics. Headless/facade coverage is provided by `frame_debug_report_summarizes_last_frame_for_editor`; backend-wgpu native stats coverage is provided by `wgpu_frame_debug_report_preserves_native_backend_stats`. | Renderer-layer editor/debug tooling is implemented through public facade commands, object association, frame/debug observability, backend-visible debug geometry, and invalid-handle errors. Host editor UI docking, input/event routing, and manipulation state are outside the renderer layer and do not remain as renderer implementation gaps. |
| Profiling/stats | Implemented | CPU timings, graph stats, pipeline stats, optional frame profile, and profiler toggle exist. Initial `RendererConfig::gpu_profiling` and runtime `enable_gpu_profiler(true)` now require `RendererFeature::TimestampQuery`; missing timestamp data keeps `gpu_time_ms` as `None` instead of reporting synthetic `0.0`, and disabled profiling suppresses any backend timestamp values from high-level stats. When GPU profiling is enabled, facade frame graph execution uses the headless RHI timestamp path and populates high-level `FrameStats::gpu_time_ms`, including graphs with imported facade texture/buffer resources mapped into headless RHI imports. Native wgpu mesh rendering now writes encoder timestamps when profiling is enabled, reports `MeshRenderStats::gpu_time_ns`, and maps that into high-level `FrameStats::gpu_time_ms` / `graph.gpu_time_ns`; covered by `initial_gpu_profiler_state_requires_timestamp_capability`, `frame_builds_stats_from_scene_and_view`, `profiler_populates_gpu_time_for_imported_environment_textures`, `profiler_populates_gpu_time_for_imported_extension_buffers`, `profiler_populates_gpu_time_for_imported_extension_textures`, `mesh_render_stats_reports_gpu_time_ms`, `wgpu_metrics_`, `renderer_config_controls_debug_label_groups`, `renderer_config_controls_transient_resource_aliasing_stats`, `cargo test -p engine_renderer gpu_profiler`, full `cargo test -p engine_renderer`, and the repeatable visible-window command `.\target\debug\render_facade_window_usecase.exe --smoke-frames 3 --wait-for-gpu --print-stats` reporting `profiler=true` and `gpu_time_ms=Some(0.26964)`. | Keep smoke launch in final verification on machines with a visible desktop. |
| Frame capture / RenderDoc hooks | External Blocked | Capture request/status/resource dump and external backend availability hooks exist. Unavailable external backends now fail at request time instead of producing a successful unavailable capture; queued external captures also report `BackendUnavailable` if the hook is removed before frame finish. A pending capture request cannot be silently overwritten by a later request. `FrameCaptureHookDesc`, `Renderer::register_frame_capture_backend_hook`, `Renderer::unregister_frame_capture_backend_hook`, `FrameCaptureIntegration`, `FrameCaptureBackendInfo::sdk_name`, `registered_hook_label`, `registered_sdk_name`, and `unavailable_reason` expose external SDK/hook metadata and user-visible unavailable reasons before a capture is queued. `RendererFeature::NativeFrameDebuggerCapture` is a reserved unsupported feature gate, and direct RenderDoc/external debugger capture requests without a registered hook return `RendererError::UnsupportedFeature(RendererFeature::NativeFrameDebuggerCapture)`. `FrameCaptureSupport::complete_native_sdk_integration` now remains true only for external backends when both availability metadata and a registered hook callback are present, so metadata-only availability does not count as complete SDK integration. The same `FrameCaptureSupport` support matrix now propagates through `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`, so capture artifacts preserve internal capture support, external-hook coverage, native-SDK blockers, unavailable backends, and complete-native-integration state; covered by `frame_capture_support_distinguishes_internal_hooks_and_native_sdk_blockers` and `frame_debug_report_summarizes_last_frame_for_editor` propagation assertions. `FrameCapture` snapshots request id, queued frame index, capture latency, backend integration, SDK name, unavailable reason, and external-hook requirement at frame finish. `FrameCapture::external_hook_triggered`, `external_hook_label`, and `external_hook_sdk_name` report whether a registered hook handoff was actually requested at frame finish and which hook/sdk was handed off. Successful registered-callback captures now persist as `FrameCaptureStatus::Captured` while panic paths remain `FrameCaptureStatus::BackendHookFailed` and removed hooks remain `BackendUnavailable`. | Built-in RenderDoc SDK / external debugger SDK loading and capture begin/end calls require repository-external SDK integration and are therefore explicitly external-blocked; current code provides the supported callback handoff integration point, capability gate, user-visible unsupported error, backend metadata, and focused capture/support tests. |
| Error handling | Implemented | Public API returns `Result` and uses validation/unsupported/invalid-handle errors. Command-buffer scene updates now reject destroyed resource handles before partial scene mutation. | Keep tightening feature gate and stale-handle errors as gaps are closed. |
| Feature flags / stability tiers | Implemented | Cargo features exist for backend-wgpu, render-graph, pbr, async, bindless, ray tracing, mesh shader, etc. Vulkan/Metal/D3D12 feature preferences are now explicitly unsupported until real backends exist. Public `RendererFeatureTier` / `RendererFeatureImplementation` / `RendererFeatureInfo` / `Renderer::feature_info` report whether each feature is `Core`, `Optional`, `Experimental`, or `ReservedUnsupported`, whether it is backend-real, facade-semantic, graph-semantic, config-gated, or reserved, plus an unsupported reason. `RendererFeature::all`, `Renderer::feature_infos`, and `RendererFeatureAudit` expose the complete runtime feature/stability list plus total/supported/unsupported, core-supported/core-unsupported, unsupported-without-reason, backend-real/facade-semantic/graph-semantic/reserved implementation counts, supported-non-backend-real feature listing, and tier counts for tools. `RendererFeatureTierSupport`, `RendererFeatureImplementationSupport`, `RendererFeatureSupportMatrix`, and `Renderer::feature_support_matrix()` now expose a product-facing matrix grouped by stability tier and implementation level, with `supported_non_backend_real_features`, `all_supported_features_backend_real`, and `all_unsupported_features_explained` keeping graph/facade semantics explicit instead of claiming backend-real coverage. `renderer_cargo_feature_infos` and `RendererCargoFeatureAudit` expose enabled/disabled state and classify all 17 `engine_renderer` Cargo features as runtime feature gates, config runtime gates, reserved backends, base facade features, or reserved tooling; covered by `renderer_feature_info_reports_tiers_and_unsupported_reasons`, `renderer_feature_infos_enumerates_all_public_features`, and `renderer_feature_support_matrix_distinguishes_backend_real_from_facade_and_graph_semantics`. | Backend-real conversion work for specific graph-semantic advanced features remains tracked by the Standard 3D graph, RHI abstraction, pipeline/cache, lighting, deformation, and synchronization rows rather than hidden in the feature matrix. |
| Complete usage examples | Implemented | `render_facade_usecase`, `render_scene_usecase`, `render_smoke`, `render_feature_showcase`, and `render_facade_window_usecase` exist. `render_facade_usecase` covers a custom RenderGraph pass. `render_facade_window_usecase` initializes a window through `Renderer::with_surface`, submits a facade `Frame::render_view`, explicitly requests GPU profiling, exposes draw/visible/GPU-time or profiler-gate state plus surface-readback materialization count in the window title, and supports repeatable smoke verification through `--smoke-frames`, `--wait-for-gpu`, `--print-stats`, `--require-gpu-time`, `--surface-readback`, and `--require-surface-readback`. The renderer example set builds together with `cargo build -p render_scene_usecase -p render_facade_usecase -p render_facade_window_usecase -p render_smoke -p render_feature_showcase`; latest targeted window example build and local visible-window smoke launch passed. | Keep visible-window smoke launch in final verification on machines with a visible desktop. |

## Current highest-priority gaps

1. Keep native RenderDoc/external debugger SDK loading/capture calls explicitly `External Blocked` until a repository-external SDK integration is linked; the current supported in-repo path is internal capture plus registered callback handoff.
2. Use the new `RenderGraphStats::semantic_passes` / `rhi_executed_passes` observability to continue converting graph-only standard pass semantics into backend-real execution where practical.
3. Extend destroyed/stale resource tests beyond the newly covered scene command-buffer, material create/update, material template shader dependencies, pipeline warmup keys, LOD dependency/frame output, deformation stats, motion-vector stats, environment graph import/output, custom graph extension imports, usage mismatches and indirect buffer imports, bindless texture table, virtual texture feedback, streaming output, texture-view/external-target frame outputs, and capture dump paths into remaining specialized graph and frame output paths.
4. Continue closing upload queue and backend GPU destruction from submitted-frame bookkeeping/explicit completed-boundary polling toward true backend fence objects/nonblocking per-submission completion queries and any remaining backend-owned tombstones, or keep explicitly marked `Partial`.
5. Run/build all API-document-aligned examples as part of final verification, including the repeatable window smoke command on machines with a visible desktop.

## 本轮 Material API 证据补充

- 能力项：Material / material template / render state。
- 本轮实现：新增公开 `MaterialInfo` 与 `Renderer::material_info`，让 material facade 可直接观察 label、domain、template handle、template readiness、standard/custom 分类、参数数量、纹理/采样器绑定数量、pipeline readiness 和资源状态。
- 闭合语义：material template 被销毁后，material 仍可通过公开 API 查询；`template_ready=false` 且 `pipeline_ready=false`，不再只能依赖 frame-time 错误定位模板依赖失效。
- 验证命令：`cargo test -p engine_renderer material_info_reports_template_bindings_and_pipeline_readiness -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。Material API 仍需继续闭合 backend material-template bind group / pipeline layout 真实接线、shader reflection 到 material schema 的完整约束、示例覆盖和 frame/capture 更完整观测。

## 本轮 Material Template API 证据补充

- 能力项：Material / material template / render state。
- 本轮实现：新增公开 `MaterialTemplateInfo` 与 `Renderer::material_template_info`，让 template facade 可直接观察 label、shader handle、shader readiness、domain、render state、schema 参数数量、pass flags、pipeline readiness 和资源状态。
- 闭合语义：shader 被销毁后，material template 仍可通过公开 API 查询；`shader_ready=false` 且 `pipeline_ready=false`。template 自身被销毁后，该 template handle 不再返回 `MaterialTemplateInfo`。
- 验证命令：`cargo test -p engine_renderer material_template_info_reports_shader_dependency_and_pipeline_readiness -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。Material API 仍需继续闭合 backend material-template bind group / pipeline layout 真实接线、shader reflection 到 material schema 的完整约束、示例覆盖和 frame/capture 更完整观测。

## 本轮 Shader Reflection / Material Schema 证据补充

- 能力项：Material / material template / render state；Shader / reflection / variants。
- 本轮实现：`create_material_template` 在 shader 提供 `ShaderInterfaceDesc.resources` 时校验 `MaterialParameterSchema`，拒绝 schema 中不存在于 shader reflection 的参数，并校验 texture / sampler / uniform / storage buffer 的 binding class 与 binding type 一致。
- 兼容语义：reflection disabled 或 interface 为空时，继续允许手写 material schema，避免把无反射 shader 错误降级为不可用。
- 验证命令：`cargo test -p engine_renderer material_template_schema_is_validated_against_shader_reflection -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合 shader reflection 自动提取、material schema 到 backend bind group / pipeline layout 的真实接线、示例覆盖和 frame/capture 观测。

## 本轮 MaterialTemplate Reflection Observability 证据补充

- 能力项：Material / material template / render state；Shader / reflection / variants。
- 本轮实现：扩展 `MaterialTemplateInfo`，公开 reflected binding 总数、texture/sampler/buffer binding 数量，以及 `schema_covers_reflection`，让 editor、pipeline warmup 和诊断路径能直接观察 material template 与 shader reflection 的覆盖关系。
- 闭合语义：reflection disabled 或 shader interface 为空时，手写 schema 继续可用；显式 reflection shader 的 template info 会报告 3 个 reflected bindings、1 个 texture、1 个 sampler、1 个 buffer，并确认 schema 覆盖 reflection。
- 验证命令：`cargo test -p engine_renderer material_template_ -- --nocapture`。
- 验证结果：3 passed。
- 剩余状态：`Partial`。仍需继续闭合自动 shader reflection 提取、backend bind group / pipeline layout 真实接线、示例覆盖和 frame/capture 观测。

## 本轮 MaterialTemplate Missing Reflection 证据补充

- 能力项：Material / material template / render state；Shader / reflection / variants。
- 本轮实现：扩展 `MaterialTemplateInfo`，公开未被 material schema 覆盖的 reflected binding 总数，并按 texture / sampler / buffer 分类统计缺口。
- 闭合语义：显式 reflection shader 可以创建只覆盖部分 reflection 的 template；该 template 仍可被查询并报告 `schema_covers_reflection=false`、`missing_reflected_bindings=2`、`missing_reflected_sampler_bindings=1`、`missing_reflected_buffer_bindings=1`，用于 editor/pipeline warmup 在 frame 前暴露绑定缺口。
- 验证命令：`cargo test -p engine_renderer material_template_ -- --nocapture`。
- 验证结果：3 passed。
- 剩余状态：`Partial`。仍需继续闭合自动 shader reflection 提取、backend bind group / pipeline layout 真实接线、示例覆盖和 frame/capture 观测。

## 本轮 Material Instance Binding Coverage 证据补充

- 能力项：Material / material template / render state；Shader / reflection / variants。
- 本轮实现：扩展 `MaterialInfo`，公开 material 实例对应 template schema 参数数量、是否覆盖 template schema、是否覆盖 shader reflection，以及 material 实例缺失 reflected binding 的总数和 texture / sampler / buffer 分类数量。
- 闭合语义：显式 reflection shader 下，template 可以只声明部分 reflection binding；material 实例可以覆盖该 template schema，同时通过 `MaterialInfo` 报告 `material_covers_reflection=false`、`missing_reflected_bindings=2`、`missing_reflected_sampler_bindings=1`、`missing_reflected_buffer_bindings=1`，用于 editor/pipeline warmup 在 frame 前暴露实例绑定缺口。
- 验证命令：`cargo test -p engine_renderer material_ -- --nocapture`。
- 验证结果：18 passed。
- 剩余状态：`Partial`。仍需继续闭合自动 shader reflection 提取、backend bind group / pipeline layout 真实接线、示例覆盖和 frame/capture 观测。

## 本轮 WGSL Auto Reflection Storage Buffer 证据补充

- 能力项：Shader / reflection / hot reload / variants；Material / material template / render state。
- 本轮实现：修正 WGSL auto reflection 的 storage buffer 分类。`var<storage>` / `var<storage, read>` 现在优先依据 address space 报告为 `BindingClass::Storage` + `BindingType::Buffer`，即使绑定类型是用户自定义 struct 而不是 `array<>` / `atomic<>`。
- 闭合语义：auto reflection 测试覆盖 uniform buffer、2D texture、sampler、cube texture 和 struct storage buffer，避免 material/template reflection 观测把 storage buffer 误分类为 uniform buffer。
- 验证命令：`cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合更多 WGSL 类型、vertex input/push constant 自动反射、backend bind group / pipeline layout 真实接线和示例覆盖。

## 本轮 WGSL Auto Reflection Push Constant 证据补充

- 能力项：Shader / reflection / hot reload / variants；Pipeline / pipeline key / cache。
- 本轮实现：WGSL auto reflection 现在解析 `var<push_constant>`，从简单 WGSL struct/scalar/vector/matrix 字段估算 byte range，并把结果写入 `ShaderInterfaceDesc.push_constants`。
- 闭合语义：`ShaderInterfaceDesc` 不再只通过 explicit reflection 暴露 push constants；Auto WGSL shader 可以在创建时获得 `PushConstantRange { stages, range }`，供 shader info、hot reload compatibility、material/template 诊断和后续 pipeline layout 接线使用。
- 验证命令：`cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合更完整 WGSL layout、vertex input 自动反射、backend pipeline layout 接线和示例覆盖。

## 本轮 WGSL File Auto Reflection Push Constant 证据补充

- 能力项：Shader / reflection / hot reload / variants。
- 本轮实现：增强 `.wgsl` 文件源码的 `ShaderReflectionMode::Auto` 覆盖，确认 `ShaderSource::File` 与内存 WGSL 字符串使用同一套 reflection 路径。
- 闭合语义：file-source WGSL 现在通过测试验证 resource binding 和 `var<push_constant>` 都进入 `ShaderInterfaceDesc`，避免自动反射只在内存源码路径可观测。
- 验证命令：`cargo test -p engine_renderer shader_file_source_is_validated_and_reflected_for_wgsl -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合更完整 WGSL layout、vertex input 自动反射、backend pipeline layout 接线和示例覆盖。

## 本轮 WGSL Auto Reflection Vertex Input 证据补充

- 能力项：Shader / reflection / hot reload / variants；Mesh / Buffer API；Pipeline / pipeline key / cache。
- 本轮实现：WGSL auto reflection 现在解析 vertex entry point 参数中的 `@location(n) name: type`，并写入 `ShaderInterfaceDesc.vertex_inputs`。
- 闭合语义：常见参数名映射到 `VertexSemantic::Position`、`Normal`、`TexCoord(0)`、`Color(0)`；未知参数名保留为 `VertexSemantic::Custom(location)`。类型映射覆盖 `vec2/3/4<f32>`、`vec2/4<u16>` 和 `u32`。
- 验证命令：`cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合更完整 WGSL 类型/layout、backend pipeline layout 接线和示例覆盖。

## 本轮 WGSL File Auto Reflection Vertex Input 证据补充

- 能力项：Shader / reflection / hot reload / variants；Mesh / Buffer API；Pipeline / pipeline key / cache。
- 本轮实现：增强 `.wgsl` 文件源码的 `ShaderReflectionMode::Auto` 测试覆盖，确认 `ShaderSource::File` 与内存 WGSL 字符串一样会把 vertex entry `@location` 参数写入 `ShaderInterfaceDesc.vertex_inputs`。
- 闭合语义：file-source WGSL 现在通过测试同时验证 resource binding、push constant range 和 vertex input requirements，避免自动反射只在内存源码路径完整。
- 验证命令：`cargo test -p engine_renderer shader_file_source_is_validated_and_reflected_for_wgsl -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合更完整 WGSL 类型/layout、backend pipeline layout 接线和示例覆盖。

## 本轮 Auto Reflection Hot Reload Compatibility 证据补充

- 能力项：Shader / reflection / hot reload / variants；Pipeline / pipeline key / cache。
- 本轮实现：新增 Auto WGSL reflection 的 hot reload 兼容性测试，验证 `validate_shader_reload_compatible` 对 Auto 产出的 `ShaderInterfaceDesc` 同样生效。
- 闭合语义：相同 resource/push-constant/vertex-input layout 的 reload 可以通过；改变 auto-reflected vertex input format 或 push constant range 的 reload 会被拒绝，防止 pipeline layout 在热重载中静默漂移。
- 验证命令：`cargo test -p engine_renderer shader_auto_reflection_hot_reload_rejects_layout_changes -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合 backend pipeline layout 接线、更多 WGSL layout 和示例覆盖。

## 本轮 WGSL Auto Reflection Vertex Entry Filtering 证据补充

- 能力项：Shader / reflection / hot reload / variants；Pipeline / pipeline key / cache。
- 本轮实现：WGSL auto reflection 的 vertex input 解析现在只针对 `ShaderEntryPoints::vertex` 指定的 vertex entry point；fragment entry 中的 `@location` 参数不会写入 `ShaderInterfaceDesc.vertex_inputs`。
- 闭合语义：避免 fragment shader 的 interpolated inputs 或 outputs 污染 pipeline vertex layout，保证 `vertex_inputs` 只描述 mesh/vertex buffer 输入需求。
- 验证命令：`cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合更完整 WGSL 类型/layout、backend pipeline layout 接线和示例覆盖。

## 本轮 WGSL Auto Reflection Vertex Input Struct 证据补充

- 能力项：Shader / reflection / hot reload / variants；Mesh / Buffer API；Pipeline / pipeline key / cache。
- 本轮实现：WGSL auto reflection 现在解析 vertex input struct 成员上的 `@location`，并在 vertex entry 参数引用该 struct 时展开到 `ShaderInterfaceDesc.vertex_inputs`。
- 闭合语义：`fn vs_main(input: VertexInput)` 现在可通过 `VertexInput` 成员反射出 `Position`、`TexCoord(0)` 和 `Joints(0)` 等 vertex input requirements；fragment entry 仍不会污染 vertex input layout。
- 验证命令：`cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合更完整 WGSL 类型/layout、backend pipeline layout 接线和示例覆盖。

## 本轮 Auto Reflection Vertex Struct Hot Reload 证据补充

- 能力项：Shader / reflection / hot reload / variants；Pipeline / pipeline key / cache。
- 本轮实现：扩展 Auto WGSL reflection hot reload 兼容性测试，覆盖 vertex input struct 展开后的 layout。
- 闭合语义：同 layout 的 struct vertex input reload 可以通过；struct 成员 `@location(0) position` 从 `vec3<f32>` 改成 `vec2<f32>` 会被拒绝，防止由 struct 展开的 pipeline vertex layout 在热重载中静默漂移。
- 验证命令：`cargo test -p engine_renderer shader_auto_reflection_hot_reload_rejects_layout_changes -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合 backend pipeline layout 接线、更多 WGSL layout 和示例覆盖。

## 本轮 WGSL Storage Texture Reflection 证据补充

- 能力项：Shader / reflection / hot reload / variants；Material / material template / render state。
- 本轮实现：WGSL auto reflection 现在把 `texture_storage_*` 分类为 `BindingClass::Storage` + `BindingType::StorageTexture { dimension, format, access }`，shader interface 校验允许 storage texture，material/template 绑定校验把 storage texture 参数按 `TextureHandle` 验证。
- 闭合语义：storage buffer 仍为 `Storage + Buffer`，storage texture 为 `Storage + StorageTexture`，避免把 storage texture 误报为 sampled texture 或要求 `Bytes` 参数。
- 验证命令：`cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`; `cargo test -p engine_renderer material_template_schema_is_validated_against_shader_reflection -- --nocapture`。
- 验证结果：2 commands passed, 1 test passed each.
- 剩余状态：`Partial`。仍需继续闭合 backend pipeline layout / bind group 接线、更多 WGSL layout 和示例覆盖。

## 本轮 WGSL D1 Texture Reflection 证据补充

- 能力项：Shader / reflection / hot reload / variants；Material / material template / render state。
- 本轮实现：WGSL texture dimension reflection 现在显式识别 `texture_1d` 和 `texture_storage_1d`，返回 `TextureDimension::D1`，避免 1D texture 默认落到 D2。
- 闭合语义：auto reflection 覆盖 sampled D1 texture 与 storage D1 texture；material/schema 绑定路径继续按 `TextureHandle` 验证 storage texture。
- 验证命令：`cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`; `cargo test -p engine_renderer material_template_schema_is_validated_against_shader_reflection -- --nocapture`。
- 验证结果：2 commands passed, 1 test passed each.
- 剩余状态：`Partial`。仍需继续闭合 backend pipeline layout / bind group 接线、更多 WGSL layout 和示例覆盖。

## 本轮 MaterialTemplate Shader Interface Layout Hash 证据补充

- 能力项：Material / material template / render state；Shader / reflection / hot reload / variants；Pipeline / pipeline key / cache。
- 本轮实现：`MaterialTemplateInfo` 新增 `shader_interface_layout_hash`，hash 覆盖 shader resources、push constants 和 vertex inputs，让 template/pipeline 诊断能观测 shader interface layout identity。
- 闭合语义：即使不进入 backend pipeline layout，公开 template info 也能看到 shader interface layout 的稳定身份；shader destroyed 后 hash 为 0，ready shader 的 empty/auto/explicit interface 都有可比较 hash。
- 验证命令：`cargo test -p engine_renderer material_template_ -- --nocapture`。
- 验证结果：3 passed。
- 剩余状态：`Partial`。仍需继续闭合 backend pipeline layout / bind group 接线、更多 WGSL layout 和示例覆盖。

## 本轮 Material Shader Interface Layout Hash 证据补充

- 能力项：Material / material template / render state；Shader / reflection / hot reload / variants；Pipeline / pipeline key / cache。
- 本轮实现：`MaterialInfo` 新增 `shader_interface_layout_hash`，直接从 material 实例当前 template 的 shader interface 计算 layout identity。
- 闭合语义：ready material 可直接暴露 shader resources、push constants 和 vertex inputs 形成的 layout hash；template 被销毁后 material 仍可查询，但 hash 变为 0，与 `template_ready=false` / `pipeline_ready=false` 一起用于 editor/pipeline 诊断。
- 验证命令：`cargo test -p engine_renderer material_info_reports_template_bindings_and_pipeline_readiness -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合 backend pipeline layout / bind group 接线、pipeline cache entry layout hash 和示例覆盖。

## 本轮 Pipeline Cache Shader Interface Layout Hash 证据补充

- 能力项：Pipeline / pipeline key / cache；Shader / reflection / hot reload / variants；Material / material template / render state。
- 本轮实现：`PipelineCacheEntryInfo` 新增 `shader_interface_layout_hash`，从 entry 的 `PipelineKey::shader` 对应 `ShaderInterfaceDesc` 计算 layout identity。
- 闭合语义：pipeline cache entry 现在可直接观测 shader resources、push constants 和 vertex inputs 形成的 layout hash，便于诊断 shader reflection/material template 与 pipeline cache entry 的对应关系；该字段不改变 cache key 行为。
- 验证命令：`cargo test -p engine_renderer pipeline -- --nocapture`。
- 验证结果：10 passed。
- 剩余状态：`Partial`。仍需继续闭合 backend pipeline layout / bind group 接线和真实 backend pipeline object 语义。

## 本轮 Pipeline Cache Shader Interface Layout Aggregate 证据补充

- 能力项：Pipeline / pipeline key / cache；Shader / reflection / hot reload / variants；Frame API / frame stats。
- 本轮实现：`PipelineCacheStats` 新增 `shader_interface_layouts`，统计当前 pipeline cache 中非零 shader interface layout hash 的唯一数量。
- 闭合语义：per-entry `PipelineCacheEntryInfo::shader_interface_layout_hash` 与 aggregate `PipelineCacheStats::shader_interface_layouts` 对齐，frame/debug stats 可以直接观察 pipeline cache 涉及多少种 shader interface layout。
- 验证命令：`cargo test -p engine_renderer pipeline -- --nocapture`。
- 验证结果：10 passed。
- 剩余状态：`Partial`。仍需继续闭合 backend pipeline layout / bind group 接线和真实 backend pipeline object 语义。

## 本轮 FrameDebugReport Pipeline Layout Aggregate 证据补充

- 能力项：Frame API / frame stats / frame capture；Debug draw / editor API；Pipeline / pipeline key / cache。
- 本轮实现：`FrameDebugReport` 新增 `pipeline_shader_interface_layouts`，顶层镜像 `PipelineCacheStats::shader_interface_layouts`。
- 闭合语义：editor/inspector 不需要解析 nested pipeline cache stats，也能直接显示最近一帧 pipeline cache 涉及多少种 shader interface layout。
- 验证命令：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合 backend pipeline layout / bind group 接线和真实 backend pipeline object 语义。

## 本轮 FrameCapture Pipeline Layout Aggregate 证据补充

- 能力项：Frame API / frame stats / frame capture；Profiling / capture / stats；Pipeline / pipeline key / cache。
- 本轮实现：`FrameCapture` 新增 `pipeline_shader_interface_layouts`，顶层镜像 `PipelineCacheStats::shader_interface_layouts`，与 `FrameDebugReport` 保持一致。
- 闭合语义：capture artifact 不需要解析 nested pipeline cache stats，也能直接暴露捕获帧 pipeline cache 涉及多少种 shader interface layout。
- 验证命令：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`。
- 验证结果：1 passed。
- 剩余状态：`Partial`。仍需继续闭合 backend pipeline layout / bind group 接线和真实 backend pipeline object 语义。

## 本轮进展：render_wgpu 材质 bind group layout 可观测
- 范围：`Render/render_wgpu` 固定 wgpu mesh material path。
- 实现：新增 `WgpuMaterialLayoutInfo`、`wgpu_material_layout_info()`、`WgpuMaterial::layout_info()`、`MeshRenderer::material_layout_info()`，公开 uniform/texture/sampler binding、占用 slot、总 binding 数和最高 binding。
- 约束：新增单元测试把公开 layout contract 与 `mesh.wgsl` 的 `@binding` 声明逐项比对，避免 shader/backend slot 漂移。
- 状态：`Partial`，backend 固定材质路径已有可观测 contract；动态材质模板到 wgpu pipeline layout/bind group 的完整接线仍未完成。

## 本轮补强：render_wgpu 材质 layout 单一来源
- 范围：`MeshRenderer::new` 创建 `Neo Material Bind Group Layout` 的实际 wgpu layout。
- 实现：实际 `wgpu::BindGroupLayout` entries 改为来自同一组 material binding contract helper，而不是独立手写大列表。
- 约束：同一个测试同时验证公开 `WgpuMaterialLayoutInfo`、实际 layout entries helper、`mesh.wgsl` 的 binding 声明。
- 状态：`Partial`，固定 wgpu material backend contract 已收敛到单一来源；动态 renderer material-template 到 backend layout/bind group 仍是后续缺口。

## 本轮补强：render_wgpu 实际 material bind group 使用 contract 常量
- 范围：`WgpuMaterial::new` 创建 `Neo Material Bind Group` 的 resource entries。
- 实现：uniform、texture、sampler resource bindings 全部改为复用 `MATERIAL_*_BINDINGS` contract 常量，和公开 info、layout entries、shader bindings 对齐。
- 验证：`cargo test -p render_wgpu material_backend_layout_info_matches_mesh_shader_bindings -- --nocapture` 通过。
- 状态：`Partial`，固定 wgpu material path 的 binding contract 已覆盖公开观测、layout 创建、resource 创建和 shader 声明；动态 renderer material-template backend 仍未完成。

## 本轮进展：wgpu backend pipeline layout inventory 进入 facade stats
- 范围：`Render/render_wgpu` 静态 pipeline inventory 与 `engine_renderer` 的 `backend_wgpu` stats 映射。
- 实现：`MeshRenderer` 新增 `STATIC_RENDER_PIPELINE_LAYOUT_COUNT` 和 `render_pipeline_layout_count()`；`backend_wgpu` 将该值写入 `FrameStats.pipeline_cache.shader_interface_layouts`。
- 影响：`FrameDebugReport.pipeline_shader_interface_layouts` 可以反映 wgpu backend 的原生 pipeline layout inventory，而不只依赖 facade/headless pipeline cache。
- 状态：`Partial`，backend inventory 统计已接入；动态 material-template 到 native pipeline layout/bind group 仍未实现完整接线。

## 本轮进展：shader resource group/binding 与 wgpu layout plan
- 范围：`engine_renderer` shader interface、WGSL auto reflection、`backend_wgpu` layout planning。
- 实现：`ShaderResourceBinding` 新增 `group`、`binding`；WGSL auto reflection 解析 `@group(n)` / `@binding(n)`；shader interface hash 纳入 group/binding；validation 拒绝重复 `(group, binding)` slot。
- Backend 接线：新增 `wgpu_shader_interface_layout_plan()`，可把 `ShaderInterfaceDesc` 转成按 group 分组、按 binding 排序的 `wgpu::BindGroupLayoutEntry` 计划和 push constant ranges。
- 状态：`Partial`，uniform/storage buffer/sampled texture/sampler/storage texture layout planning 已具备；native pipeline/bind group 对象创建还未完成。

## 本轮进展：storage texture format/access reflection 与 wgpu layout mapping
- 范围：`engine_renderer` shader interface、WGSL auto reflection、material/schema validation、`backend_wgpu` layout planning。
- 实现：新增 `BindingType::StorageTexture { dimension, format, access }` 和 `StorageTextureAccess`；`texture_storage_*<format, access>` 现在解析 format/access；layout hash 纳入 storage texture format/access。
- Backend 接线：`wgpu_shader_interface_layout_plan()` 现在可把 supported storage texture 映射为 `wgpu::BindingType::StorageTexture`，支持 `Rgba8Unorm`、`Rgba16Float`、`Rgba32Float` 与 `read`/`write`/`read_write` access；不支持的 storage format 在 layout planning 阶段返回 validation error。
- 状态：`Partial`，storage texture layout planning 缺口关闭；实际 native bind group layout/pipeline layout/pipeline/bind group 对象创建和 material 参数提交路径仍未完成。

## 本轮进展：material parameters 到 backend bind group resource plan
- 范围：`backend_wgpu` 的 material parameter -> bind group resource entry 接线前置。
- 实现：新增 `WgpuMaterialBindGroupResourcePlan`、`WgpuMaterialBindGroupResourceGroupPlan`、`WgpuMaterialBindGroupResourceEntryPlan`、`WgpuMaterialBindingResource` 和 `wgpu_material_bind_group_resource_plan()`。
- 能力：根据 `ShaderResourceBinding.group/binding` 把 material parameters 映射到 backend resource entries，按 group 归组、按 binding 排序；支持 texture、storage texture、sampler、uniform/storage buffer bytes。
- 拒绝路径：重复参数、未反射绑定的参数、参数资源类型与 shader binding 不匹配时返回 `MaterialParameterMismatch`。
- 状态：`Partial`，material 参数到 native bind group entries 的无 GPU plan 已具备；实际 `wgpu::BindGroup` resource object creation 仍未完成。

## 本轮进展：reflected layout/bind group native object creation 入口
- 范围：`backend_wgpu` 的 native object creation 前置接线。
- 实现：新增 `WgpuShaderInterfaceLayoutObjects`、`WgpuShaderBindGroupLayoutObject`、`WgpuMaterialBindGroupObject`。
- 实现：新增 `create_wgpu_shader_interface_layout_objects()`、`create_wgpu_shader_interface_layout_objects_from_plan()`，可从 reflected shader interface/layout plan 创建 native `wgpu::BindGroupLayout` 与 `wgpu::PipelineLayout`。
- 实现：新增 `create_wgpu_material_bind_groups_from_plan()`，可从 material resource plan 与 caller-provided resource resolver 创建 native `wgpu::BindGroup`，并校验 group/layout entry count 匹配。
- 状态：`Partial`，actual layout/bind group object creation 入口已具备；runtime pipeline cache 尚未调用这些入口，render pipeline object creation 与 final submission 仍未完成。

## 本轮进展：reflected render pipeline native object creation 入口
- 范围：`backend_wgpu` render pipeline object creation。
- 实现：新增 `WgpuRenderPipelineDesc`、`create_wgpu_render_pipeline()`、`WgpuRendererRuntime::create_render_pipeline()`。
- 能力：调用方提供 shader module、pipeline layout、entry points、vertex buffer layouts、color/depth format、sample count、depth write、blend state 后，可创建 native `wgpu::RenderPipeline`。
- 校验：空 vertex entry、`sample_count == 0`、fragment entry 缺 color format 会返回 `RendererError::Validation`。
- 状态：`Partial`，render pipeline native creation entry 已具备；shader module creation、pipeline cache integration、runtime resource lookup 和 final submission 仍未完成。

## 本轮进展：wgpu shader module creation 入口
- 范围：`backend_wgpu` shader module native object creation。
- 实现：新增 `create_wgpu_shader_module()` 与 `WgpuRendererRuntime::create_shader_module()`。
- 能力：支持 `ShaderSource::Wgsl` 和 `.wgsl` `ShaderSource::File` 创建 native `wgpu::ShaderModule`。
- 拒绝路径：SPIR-V/MSL/HLSL/Slang 当前返回 `ShaderCompile`，直到翻译或 backend-specific compilation 实现。
- 状态：`Partial`，WGSL native shader module creation 入口已具备；非 WGSL shader translation、pipeline cache integration 与 final submission 仍未完成。

## 本轮进展：wgpu native pipeline cache metadata/stats
- 范围：`backend_wgpu` native pipeline cache metadata 与 `PipelineCacheStats` 汇总。
- 实现：新增 `WgpuNativePipelineCacheMetadata` 与 `WgpuNativePipelineCacheEntryMetadata`。
- 能力：按 `PipelineKey` 记录 ready native backend pipeline entry、shader interface layout hash、last-used frame、used-this-frame；支持 begin-frame usage reset、单 key invalidate、clear；可生成 `PipelineCacheStats`。
- 状态：`Partial`，pipeline cache metadata/stats 层已具备；真实 `wgpu::ShaderModule`/layout/bind group/render pipeline handles 尚未放入 runtime cache，final submission 仍未完成。

## 本轮进展：WgpuRendererRuntime 接入 native pipeline cache stats
- 范围：`backend_wgpu` runtime pipeline cache metadata ownership 与 frame stats 合并。
- 实现：`WgpuRendererRuntime` 新增 `native_pipeline_cache: WgpuNativePipelineCacheMetadata`；新增 `native_pipeline_cache_stats()`、`record_native_pipeline_ready()`、`mark_native_pipeline_used()`、`invalidate_native_pipeline()`。
- 实现：`render_scene()` 发布 `FrameStats.pipeline_cache` 前会合并固定 `MeshRenderer` pipeline inventory 与 reflected native pipeline cache stats。
- 状态：`Partial`，runtime stats integration 已具备；真实 `wgpu` handle cache ownership、pipeline creation 调用链、resource lookup 与 final submission 尚未完成。

## 本轮进展：runtime reflected pipeline build-and-cache 入口
- 范围：`backend_wgpu` runtime pipeline creation invocation 与 actual handle cache ownership。
- 实现：新增 `WgpuNativePipelineObjects`，`WgpuRendererRuntime` 新增 `native_pipeline_objects` ownership map。
- 实现：新增 `insert_native_pipeline_objects()`、`native_pipeline_objects()`；`invalidate_native_pipeline()` 同时移除 metadata 与 owned handles。
- 实现：新增 `WgpuNativeRenderPipelineBuildDesc` 与 `WgpuRendererRuntime::create_and_cache_native_render_pipeline()`，组合 shader module、layout objects、render pipeline creation，并插入 runtime native pipeline handle cache。
- 状态：`Partial`，actual wgpu handle ownership 与 pipeline build-and-cache entry 已具备；runtime resource lookup、material bind group auto creation、scene/final render submission path 仍未完成。

## 本轮进展：material bind group auto creation 与 owned buffer payload
- 范围：`backend_wgpu` material parameter resource plan 到 native bind group object creation。
- 实现：`WgpuMaterialBindingResource::BufferBytes` 从仅记录长度改为保留完整 bytes payload。
- 实现：新增 `WgpuMaterialOwnedBuffer`；`WgpuMaterialBindGroupObject` 持有 `owned_buffers`，确保 bind group 引用的 uniform/storage buffer 生命周期安全。
- 实现：新增 `create_wgpu_material_bind_groups_with_owned_buffers_from_plan()`，可自动为 `Bytes` 参数创建 native `wgpu::Buffer`，并通过 caller resolver 解析 texture/sampler 后创建 `wgpu::BindGroup`。
- 实现：`WgpuNativeRenderPipelineBuildDesc` 支持 `material_resource_plan`；`WgpuRendererRuntime::create_and_cache_native_render_pipeline_with_resource_resolver()` 在 build-and-cache 流程内自动创建 material bind groups。
- 状态：`Partial`，material bind group auto creation 入口已具备；texture/sampler runtime resource table lookup 与 final submission 尚未完成。

## 本轮进展：runtime texture/sampler resource lookup 与 render-pass binding helper
- 范围：`backend_wgpu` reflected material resource lookup 与 final submission 辅助。
- 实现：新增 `WgpuMaterialExternalResourceRegistry`、`WgpuMaterialTextureBinding`、`WgpuMaterialSamplerBinding`。
- 实现：`WgpuRendererRuntime` 新增 material texture/sampler register/unregister API 与 `material_external_resources()` 查询。
- 实现：新增 `create_and_cache_native_render_pipeline_with_registered_resources()`，可使用 runtime registry 解析 texture/sampler 并自动创建 material bind groups。
- 实现：新增 `native_pipeline_objects_for_submission()`，获取 cached pipeline handles 并标记 used-this-frame；新增 `bind_wgpu_native_pipeline_for_render_pass()`，在 `wgpu::RenderPass` 中绑定 cached render pipeline 和 material bind groups。
- 状态：`Partial`，runtime resource lookup 和 render-pass binding helper 已具备；尚未把 reflected pipeline submission 自动接入 `render_scene()`/scene queue，actual GPU smoke test 也未完成。

## 本轮进展：actual wgpu reflected pipeline smoke test
- 范围：`backend_wgpu` reflected native object creation 与 render-pass binding 全链路 smoke。
- 实现：新增 `wgpu_reflected_pipeline_smoke_creates_and_binds_native_objects` 测试。
- 覆盖：真实创建 `wgpu::ShaderModule`、`wgpu::BindGroupLayout`、`wgpu::PipelineLayout`、owned uniform buffer、`wgpu::BindGroup`、`wgpu::RenderPipeline`；在 1x1 offscreen target 上 begin render pass、绑定 cached pipeline/material bind group、draw full-screen triangle、submit queue、poll device。
- 统计：测试验证 `WgpuNativePipelineCacheMetadata` 的 total/backend_objects/entries_used_this_frame/shader_interface_layouts。
- 状态：`Partial`，actual GPU smoke test 缺口关闭；尚未把 reflected pipeline submission 自动挂入 `render_scene()`/scene queue。

## 本轮进展：runtime reflected pipeline draw-to-view submission
- 范围：`backend_wgpu` reflected native pipeline final submission helper。
- 实现：新增 `WgpuNativePipelineDrawDesc`；`WgpuNativePipelineSubmissionInfo` 增加 vertex/instance count。
- 实现：新增 `WgpuRendererRuntime::submit_native_pipeline_draw_to_view()`，可从 runtime cached native pipeline 创建 command encoder/render pass、绑定 pipeline/material bind groups、draw、submit queue。
- 验证：actual wgpu smoke test 改为调用 runtime submission API，并继续通过。
- 状态：`Partial`，runtime-level reflected pipeline final submission helper 已具备；尚未自动挂入 `render_scene()`/scene queue。

## 本轮进展：render_scene queued reflected pipeline submission 接线
- 范围：`render_wgpu` MeshRenderer/WgpuRenderScene post-pass hook 与 `backend_wgpu` render_scene queued native draw integration。
- 实现：`MeshRenderer::render_batches_with_environment_probes_and_post_pass()` 在`Neo Post Process Pass` 末尾开放 post-pass hook；默认路径保持 noop。
- 实现：`WgpuRenderScene::render_with_post_pass()` 将 post-pass hook 传入同一个 surface frame，避免第二次 acquire/present。
- 实现：`WgpuRendererRuntime` 新增 `WgpuQueuedNativePipelineDraw` queue；`render_scene()` drain queued reflected draws，在 mesh pass post-hook 中绑定 cached native pipeline/material bind groups 并 draw，同时标记 native pipeline used-this-frame 并把 reflected draw count 合入 `FrameStats.draw_calls`。
- 实现注意：post-pass submission 使用 `Arc` 持有 native pipeline/bind group handles，保证 render pass 绑定期间 handle 生命周期；wgpu `RenderPass` lifetime 需要局部 unsafe 绑定 helper，安全前提被限定在同一 render pass 调用内。
- 状态：`Partial`，automatic `render_scene()` API 接线已具备；仍缺带窗口/surface 的场景级 reflected queue smoke test 与更高层 scene/material 自动排队策略。

## 本轮验证：facade custom material 到 wgpu reflected draw queue 的自动计划
- 变更：`Render/engine_renderer/src/lib.rs` 增加 facade retained-scene draw item 到 wgpu reflected native draw 的自动计划与排队路径。当前闭合范围是 WGSL reflected custom material、无 vertex input、材质 reflected resource 全部由 bytes buffer 参数提供；surface facade 渲染前会创建 native shader/layout/bind group/render pipeline，并通过 backend queued draw 在同一 swapchain frame 中提交。
- 命令：`cargo test -p engine_renderer reflected_facade -- --nocapture`
- 结果：passed，2 passed；当时覆盖 custom material 自动生成 wgpu reflected facade draw plan，以及带 mesh vertex input 的 reflected shader 返回显式 validation error；mesh vertex/index buffer binding 已在后续 reflected texture/sampler/native registration 条目闭合。
- 命令：`cargo test -p engine_renderer wgpu_reflected_pipeline_smoke_creates_and_binds_native_objects -- --nocapture`
- 结果：passed，1 passed；继续覆盖实际 wgpu shader module/layout/bind group/render pipeline 创建和 draw-to-view 提交。
- 后续闭合：facade 自动路径的 mesh vertex/index buffers、renderer texture/sampler native registration、with-surface/window-backed reflected queue smoke 已在后续条目补齐；该历史条目仅保留当时的阶段性状态。

## 本轮验证：窗口 facade usecase 接入 reflected custom material
- 变更：`Examples/render_facade_window_usecase/src/main.rs` 增加一个 procedural WGSL reflected custom material 对象，使用 renderer facade 创建 shader/material template/material，并通过 retained scene 触发自动 wgpu reflected draw queue；smoke stats 输出增加 pipeline cache total、backend object count 和 shader layout count。
- 命令：`cargo build -p render_facade_window_usecase`
- 结果：passed；example 编译到新的 facade reflected material 路径。
- 未覆盖：本轮未自动启动 GUI/surface smoke；仍需要在用户允许 GUI 启动时运行 `render_facade_window_usecase --smoke-frames 3 --wait-for-gpu --print-stats` 验证真实 swapchain frame 中的 queued reflected draw。

## 本轮验证：窗口 surface smoke 覆盖 reflected draw queue
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 增加 `WgpuRendererRuntime::surface_color_format()`，`Render/engine_renderer/src/lib.rs` 在创建 facade reflected native pipeline 时优先使用真实 runtime surface format，修复 HDR/默认 caps format 与 swapchain render pass format 不一致的问题。
- 命令：`cargo test -p engine_renderer reflected_facade -- --nocapture`
- 结果：passed，2 passed。
- 命令：`cargo build -p render_facade_window_usecase`
- 结果：passed。
- 命令：`target\debug\render_facade_window_usecase.exe --smoke-frames 3 --wait-for-gpu --print-stats`（隐藏窗口，20 秒超时保护）
- 结果：passed，exit 0；输出 `surface-smoke frame=2 draws=3 visible=2 profiler=true gpu_time_ms=Some(0.44684) graph_passes=21 rhi_passes=1 semantic_passes=0 pipeline_cache_total=2 backend_objects=25 shader_layouts=2`。
- 覆盖：真实 surface/swapchain frame 中的 facade reflected custom material 自动 pipeline 创建、queue、post-pass draw、pipeline cache/layout 统计。
- 仍未覆盖：mesh vertex/index buffer binding 的 reflected custom material；renderer texture/sampler 到 native wgpu texture view/sampler 的自动注册在后续 texture/sampler native registration 项闭合。

## 本轮验证：facade reflected material texture/sampler native registration
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 增加 `WgpuMaterialTextureUploadDesc` / `WgpuMaterialTextureUpload`，并支持由 renderer texture/sampler 描述创建 native `wgpu::Texture`、`TextureView`、`Sampler` 后注册到 material external resource registry。
- 变更：`Render/engine_renderer/src/lib.rs` 的 facade reflected draw path 现在会扫描 reflected material 参数，自动注册 Texture/Sampler 参数；native pipeline key 纳入 material handle、parameter hash 和 renderer texture revision，避免不同 material 参数或已更新 texture 内容错误复用同一个 native bind group。
- 变更：`Render/engine_renderer/src/backend_wgpu.rs` 将 structural native render pipeline object cache 与 material bind group entry cache 拆开；同一 render pipeline key 下的多个 material/texture revision key 会复用同一个 `wgpu::RenderPipeline`，`PipelineCacheStats::backend_objects` 统计 unique native render pipeline objects，并支持按 ShaderHandle / MaterialTemplateHandle / MaterialHandle 批量失效。
- 变更：`Examples/render_facade_window_usecase/src/main.rs` 的 reflected custom material 改为实际采样 renderer texture/sampler，而不是只使用 uniform bytes。
- 命令：`cargo test -p engine_renderer reflected_facade -- --nocapture`
- 结果：passed，4 passed；覆盖 procedural reflected material、texture/sampler reflected material、texture update 后 native key 刷新、mesh vertex/index buffer binding、renderer-generated mip-chain texture upload splitting。
- 命令：`cargo test -p engine_renderer wgpu_native_cache_reuses -- --nocapture`
- 结果：passed，1 passed；验证两个 material bind group cache entries 复用一个 native render pipeline object，`backend_objects == 1`，并覆盖 shader/template/material 批量失效将 native entries 移入 backend-owned tombstones，保留 shader module/layout/bind group/owned buffer/render-pipeline refs，并通过显式 poll 完成 tombstone 退休。
- 命令：`cargo test -p engine_renderer material -- --nocapture`
- 结果：passed，24 passed；覆盖 material update/schema/resource validation 及 backend cache 复用测试。
- 命令：`cargo test -p engine_renderer pipeline -- --nocapture`
- 结果：passed，14 passed；覆盖 shader reload/destroy/template destroy 的 renderer facade cache 失效路径及 backend cache 复用测试。
- 命令：`cargo build -p render_facade_window_usecase`
- 结果：passed。
- 命令：`target\debug\render_facade_window_usecase.exe --smoke-frames 3 --wait-for-gpu --print-stats`（隐藏窗口，20 秒超时保护）
- 结果：passed，exit 0；输出 `surface-smoke frame=2 draws=3 visible=2 profiler=true gpu_time_ms=Some(0.0) graph_passes=21 rhi_passes=1 semantic_passes=0 pipeline_cache_total=2 backend_objects=25 shader_layouts=2`。
- 覆盖：public renderer facade 创建 reflected shader/material template/material，material 参数包含 TextureHandle/SamplerHandle，backend-wgpu 创建 native texture view/sampler，texture update/generate-mips revision 被纳入 reflected native bind group key 以触发 bind group 重建，structural render pipeline key 保持稳定并复用 `wgpu::RenderPipeline`，renderer-generated mip-chain texture 拆成 per-mip upload，reflected shader 的 mesh vertex/index buffer 被转换成 native buffer 并在真实 swapchain frame 中 queued reflected draw 执行并进入 pipeline cache/layout stats。
- 仍未覆盖：更长期的 backend resource residency/fence-backed lifetime 管理仍在 GPU memory/upload 项下跟踪。

## 本轮进展：shader variant cache frame/debug/capture 可观测性
- 范围：Shader API、Frame API / frame stats / frame capture、Debug draw / editor API。
- 变更：`FrameStats` 新增 `shader_variant_cache_entries`、`shader_variants_used_this_frame`、`shader_variants_backend_compiled`、`shader_variant_interface_layouts`，由 renderer-layer shader variant cache 在 frame finish 时统一聚合。
- 变更：`FrameDebugReport` 和 `FrameCapture` 镜像这些字段，editor/inspector 和 capture artifact 不需要枚举所有 shader variant entries 即可看到 variant cache 压力、当帧使用量、backend 编译产物数量和涉及的 shader interface layout 数。
- 验证命令：`cargo test -p engine_renderer shader_variant_cache_tracks_features_and_invalidates_with_shader -- --nocapture`。
- 验证命令：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`。
- 状态：`Partial`。Renderer-layer variant cache API 与 frame/debug/capture 可观测性已闭合；更广泛的 backend 编译产物变体管理仍属于后续完整 renderer 层收敛项。

## 本轮进展：backend-wgpu shader variant module cache
- 范围：Shader / reflection / hot reload / variants；backend-wgpu；Frame API / frame stats / frame capture。
- 变更：`Renderer::warm_up_shader_variants` 在 wgpu runtime 存在时会通过 backend-wgpu 编译并缓存对应 variant 的 native `wgpu::ShaderModule`；重复 warmup 复用同一 module cache entry。
- 变更：`ShaderVariantInfo::backend_compiled` 暴露 warmed variant 是否已有 backend 编译产物；`FrameStats`、`FrameDebugReport` 和 `FrameCapture` 增加 `shader_variants_backend_compiled` 聚合计数。
- 变更：shader reload / destroy 会同时清理 renderer-layer variant entries，并将 backend-wgpu variant shader module cache 中的旧 `wgpu::ShaderModule` 移入 backend-owned tombstone；显式 poll 后完成退休，避免热重载后旧 shader module 直接无序释放或继续驻留 active cache。
- 验证命令：`cargo test -p engine_renderer shader_variant -- --nocapture`。
- 验证命令：`cargo test -p engine_renderer wgpu_shader_variant_module_cache -- --nocapture`。
- 状态：`Partial`。Variant 到 backend shader module 的第一段真实编译产物缓存与 tombstone 退休已闭合；后续仍需把 feature variant specialization 进一步接入 native render pipeline key / material template permutation 管理。

## 本轮进展：WGSL auto reflection vertex format coverage
- 范围：Mesh / Buffer API；Shader / reflection / hot reload / variants；RHI API / backend abstraction；backend-wgpu reflected draw path。
- 变更：public `VertexFormat` 增加 `Float32`、`Uint32x2/3/4`、`Sint32`、`Sint32x2/3/4`，补齐常见 scalar/vector WGSL vertex input 类型。
- 变更：WGSL auto reflection 现在可把 vertex entry 参数中的 `f32`、`vecN<u32>`、`i32`、`vecN<i32>` 映射到 `ShaderInterfaceDesc.vertex_inputs`，并继续保留未知语义为 `VertexSemantic::Custom(location)`。
- 变更：新增格式已接入 shader interface layout hash、facade backend-wgpu vertex layout mapping 和 RHI wgpu pipeline vertex format mapping，避免 reflection 和 backend native layout 创建能力脱节。
- 验证命令：`cargo test -p engine_renderer shader_reflection_auto_extracts_wgsl_resource_bindings -- --nocapture`。
- 验证命令：`cargo test -p engine_renderer shader -- --nocapture`。
- 状态：`Partial`。常见 f32/u32/i32 WGSL vertex input layout 已贯通；packed/normalized/float16 storage formats 已在后续 explicit vertex format 条目闭合，backend feature-gated 64-bit vertex attributes仍需后续按 renderer caps/gate 补齐。

## 本轮进展：packed/normalized/float16 explicit vertex formats
- 范围：Mesh / Buffer API；Shader / reflection / hot reload / variants；RHI API / backend abstraction；backend-wgpu reflected draw path。
- 变更：public `VertexFormat` 增加 `Uint8x2/4`、`Sint8x2/4`、`Unorm8x2/4`、`Snorm8x2/4`、`Sint16x2/4`、`Unorm16x2/4`、`Snorm16x2/4`、`Float16x2/4`。
- 变更：这些 explicit vertex storage formats 已接入 shader interface layout hash、facade backend-wgpu vertex layout mapping 和 RHI wgpu pipeline vertex format mapping；显式 shader interface 可以表达 packed/normalized/float16 mesh layout，而不需要把它们错误推断为 Float32/Uint32。
- 说明：WGSL auto reflection 不会从 `vecN<f32>` 自动猜测 normalized/packed/float16 storage format，因为同一个 shader 参数类型可由多种 storage format 提供；这些格式必须通过 explicit reflection 或 mesh layout 显式给出。
- 验证命令：`cargo test -p engine_renderer shader_reflection_accepts_explicit_interface_and_validates_entry_points -- --nocapture`。
- 验证命令：`cargo test -p engine_renderer shader -- --nocapture`。
- 状态：`Partial`。wgpu 常用非 64-bit vertex formats 已由 public API/RHI/backend mapping 表达；64-bit vertex attributes 已在后续 feature gate 条目中补齐 public gate、caps 和 unsupported path。

## 本轮进展：64-bit vertex attribute feature gate
- 范围：Mesh / Buffer API；Renderer facade config/init/caps；Feature flags / stability tiers；RHI API / backend abstraction；backend-wgpu。
- 变更：新增 `RendererFeature::VertexAttribute64Bit` 和 `RendererFeatures::VERTEX_ATTRIBUTE_64BIT`，作为 optional backend-real capability。
- 变更：`graphics_wgpu` 会在 adapter 支持 `wgpu::Features::VERTEX_ATTRIBUTE_64BIT` 时请求启用该 device feature；backend-wgpu caps 仅在 device feature 已启用时报告 `VERTEX_ATTRIBUTE_64BIT`。
- 变更：public `VertexFormat` 增加 `Float64`、`Float64x2/3/4`，并接入 shader interface layout hash、facade backend-wgpu vertex layout mapping 和 RHI wgpu pipeline mapping。
- 变更：`create_shader` / `reload_shader_from_desc` 会在 shader interface 使用 `Float64*` vertex formats 但 renderer caps 不支持 `VERTEX_ATTRIBUTE_64BIT` 时返回 `UnsupportedFeature(RendererFeature::VertexAttribute64Bit)`；测试覆盖 unsupported path 和手动开启 caps 后的支持路径。
- 变更：backend-wgpu 增加 `wgpu_float64_vertex_attribute_pipeline_smoke_is_cap_gated`，当前设备不支持 `VERTEX_ATTRIBUTE_64BIT` 时断言 renderer caps 同步不支持并跳过；支持设备上会实际创建含 `wgpu::VertexFormat::Float64` vertex buffer layout 的 native render pipeline。
- 验证命令：`cargo test -p engine_renderer shader_reflection_accepts_explicit_interface_and_validates_entry_points -- --nocapture`。
- 验证命令：`cargo test -p engine_renderer renderer_feature -- --nocapture`。
- 验证命令：`cargo test -p engine_renderer wgpu_float64_vertex_attribute_pipeline_smoke_is_cap_gated -- --nocapture`（当前环境 device feature unavailable，走 caps-gated skip）。
- 状态：`Partial`。64-bit vertex attributes 已有 public gate、caps、unsupported path、backend mapping 和条件 native pipeline smoke；仍需在真实支持该 device feature 的硬件/驱动环境中记录支持路径执行结果。




## 本轮进展：backend-wgpu material external resource tombstones
- 范围：GPU memory/upload/streaming；backend-wgpu reflected material resources。
- 变更：`WgpuRendererRuntime::unregister_material_texture_binding` 和 `unregister_material_sampler_binding` 现在会把 native texture view / sampler binding 移入 backend-owned tombstone，而不是直接丢弃。
- 验证命令：`cargo test -p engine_renderer wgpu_material_external_resources_unregister_into_backend_tombstones -- --nocapture`。
- 结果：passed，1 passed；覆盖 material external texture/sampler unregister 后 tombstone 计数与显式 poll retirement。


## 本轮进展：renderer-level backend tombstone observability
- 范围：GPU memory/upload/streaming；Frame API / stats / capture。
- 变更：新增 renderer-level `BackendResourceRetirementStats`，并通过 `MemoryStats.backend_retirement`、`ResourceRetirementStats.memory` 和 `FrameCaptureResourceDump.backend_retirement` 暴露 backend-wgpu tombstone live/retired counts。
- 变更：`Renderer::poll_resource_retirements()` 现在会驱动 backend-wgpu `poll_backend_resource_retirements()`，让高层 poll 同时完成 facade resource retirement 和 backend tombstone retirement。
- 验证命令：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture`。
- 结果：passed，1 passed；覆盖 renderer-level `MemoryStats`、capture dump 和 high-level poll 对 backend tombstone retirement 的观测。

## 本轮进展：backend tombstone fence object observability
- 范围：GPU memory/upload/streaming；backend-wgpu resource lifetime。
- 变更：backend-owned tombstone 现在记录 backend fence object，并在可用时携带最新 `wgpu::SubmissionIndex`；`BackendResourceRetirementStats` 增加 live/retired fence object 计数。
- 验证命令：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture`。
- 验证命令：`cargo test -p engine_renderer wgpu_material_external_resources_unregister_into_backend_tombstones -- --nocapture`。
- 验证命令：`cargo test -p engine_renderer wgpu_shader_variant_module_cache -- --nocapture`。
- 验证命令：`cargo test -p engine_renderer wgpu_native_cache_reuses -- --nocapture`。
- 结果：均 passed；覆盖 renderer-level 和 backend-wgpu tombstone live/retired fence object 统计。

## 本轮进展：frame-begin backend tombstone maintenance
- 范围：GPU memory/upload/streaming；Frame API / stats。
- 变更：`Renderer::begin_frame()` 会自动执行一次非阻塞 backend tombstone maintenance；空帧不再必须显式调用 `poll_resource_retirements()` 才能推进已完成的 backend tombstone retirement。
- 验证命令：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture`。
- 结果：passed，1 passed；覆盖空 frame 自动退休 backend tombstone，并在 `FrameStats.memory.backend_retirement` 中报告 retired shader variant module/fence object 计数。

## 本轮进展：backend-wgpu post-pass buffer tombstones
- 变更：backend-wgpu reflected post-pass draw 创建的临时 vertex/index `wgpu::Buffer` 现在会在提交后移入 backend-owned tombstone，而不是在 `render_scene()` 返回时立即释放。
- 可观测性：`WgpuBackendResourceRetirementStats` 与 renderer-level `BackendResourceRetirementStats` 增加 live/retired post-pass vertex/index buffer 计数，并通过 `MemoryStats.backend_retirement`、`ResourceRetirementStats::memory` 和 capture dump 路径向工具层暴露。
- 验证：`cargo test -p engine_renderer wgpu_post_pass_buffers_enter_backend_tombstones_until_poll_retirement -- --nocapture` passed，覆盖 post-pass buffer tombstone 入队、live stats 和显式 poll retirement。
- 状态：GPU memory/upload/streaming 的 backend-owned tombstone 覆盖面继续扩大，但 finer per-fence non-blocking completion 和剩余 backend-owned resource classes 仍未闭合，renderer goal 不能标记完成；cooperative background retirement startup/observability 已由后续条目关闭。
- 回归验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed。
- 回归验证：`cargo test -p engine_renderer wgpu_native_cache_reuses -- --nocapture` passed。
- 新增映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed。

## 本轮进展：backend-wgpu material external resource replacement tombstones
- 范围：GPU memory/upload/streaming；backend-wgpu resource lifetime。
- 变更：`register_material_texture_binding`、`register_material_sampler_binding`、`create_and_register_material_texture_binding`、`create_and_register_material_sampler_binding` 在同一 handle 上替换已有 native texture/sampler binding 时，会把旧 binding 移入 backend-owned tombstone，而不是由 HashMap replacement 立即释放。
- 验证：`cargo test -p engine_renderer wgpu_material_external_resources_replace_into_backend_tombstones -- --nocapture` passed，覆盖 texture/sampler replacement 后 live tombstone stats 和显式 poll retirement。
- 回归验证：`cargo test -p engine_renderer wgpu_material_external_resources_unregister_into_backend_tombstones -- --nocapture` passed，确认 unregister tombstone 路径未回退。
- 状态：backend-owned material external resource lifetime 现在覆盖 unregister 与 replacement；GPU memory/upload/streaming 仍保留 Partial，因为 finer per-fence non-blocking completion 和其他 backend resource 类别仍未全部闭合；cooperative background retirement startup/observability 已由后续条目关闭。
- 补充验证：`cargo test -p engine_renderer wgpu_material_sampler_create_and_register_replaces_into_backend_tombstone -- --nocapture` passed，覆盖 create-and-register sampler replacement tombstone 路径。
- 补充验证：`cargo test -p engine_renderer wgpu_material_texture_create_and_register_replaces_into_backend_tombstone -- --nocapture` passed，覆盖 create-and-register texture replacement tombstone 路径。

## 本轮进展：backend tombstone fence-index observability
- 范围：GPU memory/upload/streaming；backend-wgpu resource lifetime；Frame stats observability。
- 变更：backend-wgpu tombstone retirement stats 现在区分带 `wgpu::SubmissionIndex` 的 fence object 和没有 submission index 的 fence object，分别暴露 live/retired 计数：`fence_submission_indices`、`fence_objects_without_submission_index`、`retired_fence_submission_indices_this_poll`、`retired_fence_objects_without_submission_index_this_poll`。
- 变更：renderer-level `BackendResourceRetirementStats` 同步映射这些字段，工具层可以通过 `MemoryStats.backend_retirement` 判断 tombstone 是真实 submission-index-backed，还是只能退化为队列空轮询。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，覆盖 indexed/unindexed tombstone fence live stats 与 poll retirement stats。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，确认 renderer-level stats 不丢字段。
- 状态：这提升了 per-fence 可观测性，但仍不是完整 finer per-fence non-blocking completion；当前可观测到 submission-index-backed tombstone 仅在有活动 completion tracker 时支持 true nonblocking retirement，无 tracker 时这些路径回退到 queue-empty fallback；未带 index 的 tombstone 仍需 queue-empty 作为安全后备。
- 高层观测验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，覆盖 renderer-level memory/capture dump 对 indexed/unindexed fence 统计的观测。
- 自动维护高层验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，覆盖空帧自动 backend tombstone maintenance 的 indexed/unindexed fence 统计观测。
- Debug report 观测验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，覆盖 `FrameDebugReport` 对自动 backend tombstone maintenance fence 细分统计的镜像。
- FrameCapture 观测验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，覆盖 `FrameCapture.resource_dump.backend_retirement` 对自动 backend tombstone maintenance fence 细分统计的镜像。

## 本轮进展：backend tombstone queue-empty poll gate observability
- 范围：GPU memory/upload/streaming；backend-wgpu resource lifetime；Frame stats/capture/debug observability。
- 变更：backend-wgpu 和 renderer-level `BackendResourceRetirementStats` 增加 `last_poll_queue_empty` 与 `retired_after_queue_empty_poll`，工具层可以区分“最近一次 non-blocking poll 已观察到 queue empty 并退休 tombstone”和“尚未满足 backend retirement gate”。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，覆盖 backend 显式 poll 的 queue-empty gate 统计。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，确认 renderer-level stats 不丢字段。
- 高层观测验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，覆盖 `FrameStats`、`FrameCapture.resource_dump` 和 `FrameDebugReport` 对 queue-empty gate 统计的镜像。
- 状态：该项让当前安全 retirement gate 变为用户可见；它仍不等同完整 finer per-fence non-blocking completion。当前 tracker 不可见时，backend 仅能通过 queue-empty 条件退休 tombstone；有 tracker 时，submission-index-backed tombstone 可通过 true nonblocking 观察到完成并退休，而不带 index 的 tombstone 仍依赖 queue-empty fallback。

## 本轮进展：backend tombstone queue-empty gate invalidation
- 范围：GPU memory/upload/streaming；backend-wgpu resource lifetime observability。
- 变更：所有 backend-owned tombstone 入队路径现在都会失效旧的 `last_poll_queue_empty` / `retired_after_queue_empty_poll` gate 状态，避免工具把“新 tombstone 入队前的一次 queue-empty poll”误认为覆盖了当前 pending tombstone set。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_enqueue_invalidates_previous_queue_empty_gate -- --nocapture` passed，覆盖先 idle poll、再入队新 tombstone、live stats gate 失效、下一次 poll 退休后 gate 重新为 true。
- 回归验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed。
- 回归验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed。
- 状态：queue-empty gate 语义更准确，但仍不是完整 per-fence non-blocking completion。

## 本轮进展：backend tombstone completed-submission-index gate observability
- 范围：GPU memory/upload/streaming；backend-wgpu resource lifetime observability。
- 变更：backend-wgpu 和 renderer-level `BackendResourceRetirementStats` 增加 `last_poll_completed_submission_index_recorded` 与 `retired_after_completed_submission_index_poll`，用于区分 queue-empty retirement poll 是否绑定到已记录的 backend submission index。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，覆盖 unindexed tombstone poll 与 indexed tombstone poll 的差异。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed。
- 高层观测验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed。
- 自动维护验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed。
- 状态：该项提升 completed-submission-index gate 可观测性，但不等同完整 finer per-fence non-blocking completion；当前 backend-wgpu 可以在有活动 completion tracker 时进行非阻塞单提交索引查询，无 tracker 时回退 queue-empty。
- 补充验证：`cargo test -p engine_renderer wgpu_backend_tombstone_enqueue_invalidates_previous_completed_submission_gate -- --nocapture` passed，覆盖 completed-submission-index gate invalidation。

## 本轮进展：completed-submission-index gate false-positive fix
- 范围：GPU memory/upload/streaming；backend-wgpu tombstone retirement observability。
- 修复：`retired_after_completed_submission_index_poll` 现在只在本次 retired tombstone set 中确实存在带 submission-index fence 的 tombstone 时置 true；如果 tombstone 入队时没有 submission index，即使 poll 前 runtime 后来记录了其他 submission index，也不会误报为 completed-submission-index retirement。
- 验证：`cargo test -p engine_renderer wgpu_backend_unindexed_tombstone_does_not_claim_completed_submission_retirement -- --nocapture` passed，覆盖 unindexed tombstone + later unrelated submission 的 false-positive 防护。
- 回归验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed。
- 状态：completed-submission-index gate observability 更准确，但仍不等同完整 per-fence non-blocking completion。

## 本轮进展：tombstone-level indexed/unindexed fence coverage stats
- 范围：GPU memory/upload/streaming；backend-wgpu tombstone observability。
- 变更：backend-wgpu 与 renderer-level `BackendResourceRetirementStats` 增加 tombstone-level indexed/unindexed 计数：`tombstones_with_submission_index`、`tombstones_without_submission_index`、`retired_tombstones_with_submission_index_this_poll`、`retired_tombstones_without_submission_index_this_poll`。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，覆盖 live/retired indexed 与 unindexed tombstone 计数。
- false-positive 回归：`cargo test -p engine_renderer wgpu_backend_unindexed_tombstone_does_not_claim_completed_submission_retirement -- --nocapture` passed。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，确认 renderer-level stats 不丢字段。
- 状态：该项提升 tombstone set coverage observability；完整 per-fence non-blocking completion 仍未实现。

## 本轮进展：all-tombstones submission-index coverage flags
- 范围：GPU memory/upload/streaming；backend-wgpu tombstone observability。
- 变更：backend-wgpu 与 renderer-level `BackendResourceRetirementStats` 增加 `all_tombstones_have_submission_index` 和 `retired_all_tombstones_had_submission_index_this_poll`，工具可以直接判断 pending/retired tombstone set 是否全部具备 submission-index fence。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，覆盖 unindexed false 与 indexed true。
- false-positive 回归：`cargo test -p engine_renderer wgpu_backend_unindexed_tombstone_does_not_claim_completed_submission_retirement -- --nocapture` passed。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed。
- 状态：该项继续强化 resource lifetime observability；完整 per-fence non-blocking completion 与后台 retirement 仍未完成。
- 高层观测验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，覆盖 frame stats / capture dump / debug report 对 all-indexed tombstone coverage 字段的镜像。
- 显式 poll 高层观测验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，覆盖 memory stats / explicit retirement poll 对 tombstone-level coverage 字段的镜像。
- mixed set 语义验证：`cargo test -p engine_renderer wgpu_backend_mixed_tombstone_set_reports_partial_submission_index_coverage -- --nocapture` passed，覆盖 indexed/unindexed tombstone 混合 set 的 partial coverage 统计。

## 本轮进展：partial submission-index coverage flags
- 范围：GPU memory/upload/streaming；backend-wgpu tombstone observability。
- 变更：backend-wgpu 与 renderer-level `BackendResourceRetirementStats` 增加 `partial_tombstone_submission_index_coverage` 和 `retired_partial_tombstone_submission_index_coverage_this_poll`，用于直接表达 pending/retired tombstone set 同时包含 indexed 与 unindexed tombstone。
- 验证：`cargo test -p engine_renderer wgpu_backend_mixed_tombstone_set_reports_partial_submission_index_coverage -- --nocapture` passed，覆盖 mixed set 的 partial flag。
- 回归验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，覆盖全 indexed / 全 unindexed 非 partial。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed。
- 状态：该项提升 tooling observability；完整 per-fence non-blocking completion 仍未完成。

## 本轮进展：no-indexed tombstone coverage flags
- 范围：GPU memory/upload/streaming；backend-wgpu tombstone observability。
- 变更：backend-wgpu 与 renderer-level `BackendResourceRetirementStats` 增加 `no_tombstones_have_submission_index` 和 `retired_no_tombstones_had_submission_index_this_poll`，与 all/partial flags 共同直接表达 full indexed、mixed indexed/unindexed、no indexed 三态。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，覆盖全 indexed 与全 unindexed。
- mixed 回归：`cargo test -p engine_renderer wgpu_backend_mixed_tombstone_set_reports_partial_submission_index_coverage -- --nocapture` passed。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed。
- 状态：该项继续强化 tooling observability；完整 per-fence non-blocking completion 仍未完成。
- 高层观测验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，覆盖 memory stats / explicit retirement poll 对 no-indexed coverage 字段的镜像。
- 自动维护高层验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，覆盖 frame stats / capture dump / debug report 对 no-indexed coverage 字段的镜像。

## 本轮进展：tombstone submission-index coverage enum
- 范围：GPU memory/upload/streaming；backend-wgpu tombstone observability；renderer tooling API。
- 变更：backend-wgpu 增加 `WgpuTombstoneSubmissionIndexCoverage`，renderer-level 增加 `TombstoneSubmissionIndexCoverage`，并在 `BackendResourceRetirementStats` 中暴露 live/retired coverage enum：`NotApplicable`、`None`、`Partial`、`All`。
- 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences -- --nocapture` passed，覆盖 `None` 与 `All`。
- mixed 验证：`cargo test -p engine_renderer wgpu_backend_mixed_tombstone_set_reports_partial_submission_index_coverage -- --nocapture` passed，覆盖 `Partial`。
- 映射验证：`cargo test -p engine_renderer renderer_backend_retirement_stats_map_post_pass_buffers -- --nocapture` passed，确认 backend enum 映射到 renderer-level enum。
- 状态：这是 tooling observability API，不是完整 per-fence non-blocking completion；renderer goal 仍未完成。
- 高层观测验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，覆盖 memory stats / explicit retirement poll 对 coverage enum 的镜像。
- 自动维护高层验证：`cargo test -p engine_renderer begin_frame_automatically_polls_backend_tombstone_retirement -- --nocapture` passed，覆盖 frame stats / capture dump / debug report 对 coverage enum 的镜像。
- NotApplicable 语义验证：`cargo test -p engine_renderer wgpu_backend_tombstone_enqueue_invalidates_previous_queue_empty_gate -- --nocapture` passed，覆盖 zero tombstone set 的 coverage enum。
- 高层 NotApplicable 验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，覆盖 high-level memory stats 的 zero tombstone coverage enum。
- idle poll reset 验证：`cargo test -p engine_renderer wgpu_backend_tombstone_enqueue_invalidates_previous_queue_empty_gate -- --nocapture` passed，覆盖 backend retired coverage enum 的 idle reset。
- 高层 idle poll reset 验证：`cargo test -p engine_renderer renderer_memory_stats_expose_backend_tombstone_retirement -- --nocapture` passed，覆盖 explicit retirement poll 的 high-level retired coverage reset。
- enum helper 直接验证：`cargo test -p engine_renderer wgpu_tombstone_submission_index_coverage_enum_covers_all_states -- --nocapture` passed，覆盖 coverage enum 四态映射。

## 2026-05-19 本轮进展：后台 resource retirement 能力边界显式化

- `RendererFeature::BackgroundResourceRetirement` 和 `RendererFeatures::BACKGROUND_RESOURCE_RETIREMENT` 已加入 public capability 体系。
- 历史状态：此条目最初将 `Renderer::start_background_resource_retirement()` 记录为 unsupported-only 边界。
- 当前状态：后续 2026-05-20 条目已实现 cooperative background retirement startup/observability，包括 lightweight scheduler thread、start/stop API、safe-point tick consumption、feature/caps support 和 memory/retirement stats active-state observability。
- 剩余边界：true nonblocking backend completion queries 在有活动 completion tracker 时可用；无活动 tracker 时当前 wgpu backend 仍以 queue-empty fallback 作为稳定 retirement gate。

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
- 同一提交流水线下可见的 tracker 行为由 `last_submission_index` 驱动：存在活动 completion tracker 时 `nonblocking_submission_index_poll_supported` 可转为 `true`，无活动 tracker 时回落到 queue-empty；`public poll_resource_retirements()` 仍可稳定使用 queue-empty 作为保守后备完成边界。
- 内部 per-fence order 过滤仍保留，用于在可提供 completed order 的路径上只释放自身 fence 已完成的 tombstone；公开 stats 不再误导为已具备真正非阻塞 per-submission 查询。
- 新增/回归验证：`wgpu_backend_retirement_filters_tombstones_by_completed_submission_index`、`renderer_backend_retirement_stats_map_post_pass_buffers`、`renderer_memory_stats_expose_backend_tombstone_retirement`。
- 状态：GPU memory / upload / delayed destroy 仍为 `Partial`；poll 粒度与限制已公开闭合，真实非阻塞 submission-index 查询在有活动 tracker 时可用，无 tracker 时回退 queue-empty，后台 worker 已接入。

## 2026-05-19 本轮进展：非阻塞 submission-index retirement poll capability gate

- public `RendererFeature::NonblockingResourceRetirementPoll` 与 `RendererFeatures::NONBLOCKING_RESOURCE_RETIREMENT_POLL` 已加入统一 feature/capability 体系。
- 当前 headless/wgpu 路径在无活动 completion tracker 时不声明该 capability；`feature_info()` 返回 `supported = false`、`implementation = ConfigGate`，reason 明确为当前 wgpu backend 在该状态使用 queue-empty fallback；当观察到可用 tracker 时会转为可用状态并报告 true 非阻塞 submission-index polling。
- 该 gate 与 `BackendResourceRetirementStats::{nonblocking_submission_index_poll_supported, queue_empty_poll_fallback, last_poll_used_queue_empty_fallback}` 对齐，用户可同时从 feature API 和 stats/capture 观察能力边界。
- 验证：`renderer_feature`、`background_resource_retirement`、`renderer_memory_stats_expose_backend_tombstone_retirement` 相关测试通过。
- 状态：GPU memory / upload / delayed destroy 仍为 `Partial`；能力 gate 已闭合，真实非阻塞 per-submission 完成查询在有活动 tracker 时可用，无 tracker 时 `supported = false` 且回退到 queue-empty。

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

## 2026-05-19 增量矩阵更新：backend-wgpu Neo GBuffer Pass

- Standard 3D graph / GBuffer：backend-wgpu 现在有真实 `Neo GBuffer Pass`，位于 `Neo Depth Prepass` 之后、`Neo Forward Opaque Pass` 之前，写入 transient offscreen render target 并绘制 opaque batches。
- 观测证据：`MeshRenderStats::gbuffer_draw_call_count`、`FrameStats::backend_gbuffer_passes`、`FrameStats::backend_gbuffer_draw_calls`、`FrameProfile`、`FrameDebugReport`、`FrameCapture` 和 `BackendNativePassDrawStats { pass_label: "Neo GBuffer Pass" }` 均可观察该 native pass。
- 覆盖证据：`annotate_backend_standard_pass_coverage()` 已将 facade standard `gbuffer` 映射到 backend native `Neo GBuffer Pass`；`default_wgpu_pass_labels_match_native_render_pass_order` 覆盖 fallback label order。
- 当前状态调整：GBuffer backend-native pass 从 label-only / headless semantic 提升为 Partial backend implementation。仍不是完整 deferred renderer，因为尚缺 MRT albedo/normal/material attachments、deferred lighting 对 GBuffer 的采样、GBuffer resource export/import、以及与 post-process family 的完整数据流。
- 验证：本轮通过 `wgpu_metrics_`、`mesh_render_stats_`、`default_wgpu_pass_labels_match_native_render_pass_order`、`facade_backend_graph_merge_preserves_semantic_and_native_execution_stats`、`wgpu_frame_debug_report_preserves_native_backend_stats`、`backend_native_pass_draw_stats_counts_repeated_pass_instances`、`frame_debug_report_summarizes_last_frame_for_editor`、`frame_builds_stats_from_scene_and_view`。

## 2026-05-19 增量矩阵更新：GBuffer MRT backend pass

- Standard 3D graph / GBuffer：状态从 backend-native single-target Partial 提升为 backend-native MRT Partial。`Neo GBuffer Pass` 现在写入 albedo/normal/material 三个 transient render targets，并使用专用 `gbuffer.wgsl` 与 4 条 GBuffer native pipelines。
- Pipeline / cache：`MeshRenderer::STATIC_RENDER_PIPELINE_COUNT` 更新为 32，pipeline inventory 和 backend-wgpu cache merge 测试已覆盖该数量，不再依赖旧的 28 条 pipeline 断言。
- Shader validation：`post_process.wgsl` 修复 naga 不支持的动态数组索引，`render_smoke.exe` hidden launch 已验证当前 shader/pipeline set 能通过真实 wgpu pipeline creation。
- 仍未完成：GBuffer attachments 还没有被 deferred lighting pass 采样；GBuffer resource export/import、lighting resolve、post-process family、external frame debugger SDK 和完整 backend resource lifetime 仍是完整 renderer goal 阻塞项。

## 2026-05-19 deferred lighting backend-wgpu coverage update

Status change: `deferred_lighting` now has a real backend-wgpu native pass after `gbuffer`.

Evidence:

- `Neo GBuffer Pass` writes sampleable single-sample MRT textures for albedo, normal, and material; MSAA render targets resolve into those sampleable textures when sample count is greater than one.
- `Neo Deferred Lighting Pass` samples all three GBuffer MRT textures in `deferred_lighting.wgsl` and writes a transient backend lighting target.
- `MeshRenderStats`, `FrameStats`, `FrameProfile`, `FrameDebugReport`, `FrameCapture`, `backend_native_pass_draws`, default backend pass labels, and facade/backend standard pass coverage expose the native deferred lighting pass.
- Verified with targeted render_wgpu and engine_renderer tests plus `render_smoke` build and hidden launch.

Matrix note: this closes the backend-wgpu sampling/observability slice for deferred lighting. It does not close the full standard 3D renderer goal because the deferred lighting target is still transient and is not yet consumed as final frame output or post-process input.

## 2026-05-19 sampled post-process bridge update

The deferred lighting output is no longer a write-only transient target. `Neo Post Process Pass` now uses a sampled post-process pipeline when `Neo Deferred Lighting Texture` is available, samples that texture through `post_process_sampled.wgsl`, applies a simple tonemap/gamma step, and alpha-blends the result into the final surface. This closes the minimal backend-wgpu path from GBuffer MRT -> deferred lighting -> post-process -> frame surface.

Implementation note: the depth prepass now explicitly binds the shadow bind group at group 2 so all pipelines using the shared mesh layout have compatible bind group state during real wgpu validation.

## 2026-05-19 tonemap backend-native coverage update

`tonemap` is now mapped to a real backend-wgpu native pass only when the sampled deferred-lighting post-process path is active. In that path, the final render pass label is `Neo Tonemap Post Process Pass`, which samples `Neo Deferred Lighting Texture` through `post_process_sampled.wgsl`, applies the tonemap/gamma step, and blends the result into the surface.

Coverage distinction:

- `Neo Tonemap Post Process Pass` covers semantic `tonemap`, `post_process_resolve`, and `present`.
- `Neo Post Process Pass` still covers `post_process_resolve` and `present` for the no-deferred-source fallback path, but it does not claim `tonemap` coverage.

This avoids treating a plain post-process/present pass as tonemap when no deferred lighting source is available.

## 2026-05-19 FXAA backend-native coverage update

`fxaa` now has a minimal real backend-wgpu sampled post-process path when facade `ViewQualitySettings::fxaa` is enabled. The facade forwards that quality flag to backend-wgpu through `WgpuPostProcessOptions`, and the sampled post-process shader uses the flag to run a simple FXAA pass before tonemap/gamma output.

Coverage distinction:

- `Neo Fxaa Tonemap Post Process Pass` covers semantic `fxaa`, `tonemap`, `post_process_resolve`, and `present`.
- `Neo Tonemap Post Process Pass` continues to cover semantic `tonemap`, `post_process_resolve`, and `present` when FXAA is disabled.
- `Neo Post Process Pass` remains the no-deferred-source fallback and only covers `post_process_resolve` and `present`.

This closes the minimal facade-to-backend FXAA switch and native label observability path. It does not close the full post-process family; bloom, TAA output integration, color grading LUT, SSR, depth of field, and motion blur still remain separate renderer-goal gaps unless independently implemented and verified.

Validation performed for this update:

- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`
- `cargo test -p render_wgpu sampled_post_process_shader_samples_deferred_lighting_target -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

## 2026-05-19 Bloom backend-native coverage update

`bloom` now has a minimal real backend-wgpu sampled post-process path when facade `ViewQualitySettings::bloom` is enabled. The facade forwards that quality flag to backend-wgpu through `WgpuPostProcessOptions`, and `post_process_sampled.wgsl` uses the flag to add a small HDR bright-neighbor contribution before tonemap/gamma output.

Coverage distinction:

- `Neo Bloom Tonemap Post Process Pass` covers semantic `bloom`, `tonemap`, `post_process_resolve`, and `present`.
- `Neo Bloom Fxaa Tonemap Post Process Pass` covers semantic `bloom`, `fxaa`, `tonemap`, `post_process_resolve`, and `present`.
- `Neo Fxaa Tonemap Post Process Pass` continues to cover `fxaa`, `tonemap`, `post_process_resolve`, and `present` when bloom is disabled.
- `Neo Tonemap Post Process Pass` continues to cover `tonemap`, `post_process_resolve`, and `present` when both bloom and FXAA are disabled.

This closes the minimal facade-to-backend bloom switch and native label observability path. It is not a complete production bloom chain: there is still no multi-resolution threshold/downsample/blur/upsample chain, no artist-facing bloom parameters, and no separate bloom texture output.

Validation performed for this update:

- `cargo test -p render_wgpu sampled_post_process -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

## 2026-05-19 Color grading backend-native coverage update

`color_grading` now has a minimal real backend-wgpu sampled post-process path when facade `ViewQualitySettings::color_grading` is `ColorGradingMode::Lut`. The facade forwards that mode as a backend post-process option, and `post_process_sampled.wgsl` applies a small post-tonemap color grading curve before gamma output.

Coverage distinction:

- `Neo Tonemap Color Grading Post Process Pass` covers semantic `color_grading`, `tonemap`, `post_process_resolve`, and `present`.
- `Neo Fxaa Tonemap Color Grading Post Process Pass` covers `fxaa`, `color_grading`, `tonemap`, `post_process_resolve`, and `present`.
- `Neo Bloom Tonemap Color Grading Post Process Pass` covers `bloom`, `color_grading`, `tonemap`, `post_process_resolve`, and `present`.
- `Neo Bloom Fxaa Tonemap Color Grading Post Process Pass` covers `bloom`, `fxaa`, `color_grading`, `tonemap`, `post_process_resolve`, and `present`.

This closes the minimal facade-to-backend color grading switch and native label observability path. It is not a complete LUT implementation: there is still no user-supplied 3D LUT texture, LUT resource lifecycle, LUT sampling shader path, or artist-facing grading parameter API.

Validation performed for this update:

- `cargo test -p render_wgpu sampled_post_process -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

## 2026-05-19 remaining post-process flags backend-visible sampled paths

`taa`, `motion_blur`, `ssr`, and `depth_of_field` now have minimal backend-wgpu sampled post-process branches when the corresponding facade `ViewQualitySettings` flags are enabled. The facade forwards these flags through `WgpuPostProcessOptions`; `post_process_sampled.wgsl` packs them into `effect_flags` and executes small single-pass sampled resolve branches before FXAA/tonemap/color grading.

Native pass labels are now generated dynamically from enabled post-process effects, for example:

- `Neo Bloom Taa Fxaa Motion Blur Ssr Depth Of Field Tonemap Color Grading Post Process Pass`

Backend/facade standard pass coverage maps these dynamic labels back to semantic `taa`, `motion_blur`, `ssr`, `depth_of_field`, `bloom`, `fxaa`, `tonemap`, `color_grading`, `post_process_resolve`, and `present` based on label tokens. `MeshRenderStats` now stores native pass label snapshots as owned strings so combined backend labels are preserved without leaking static strings.

This is still not complete production post-processing:

- `taa` lacks previous-frame history, velocity reprojection, jitter management, and neighborhood clamping.
- `motion_blur` lacks velocity-buffer driven per-pixel blur.
- `ssr` lacks depth/normal ray marching, hierarchical depth, and reflection resolve.
- `depth_of_field` lacks CoC generation, foreground/background separation, and blur chain.

Validation performed for this update:

- `cargo test -p render_wgpu sampled_post_process -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

## 2026-05-19 SSAO backend-visible sampled path

`ssao` now has a minimal backend-wgpu sampled post-process branch when facade `ViewQualitySettings::ssao` is enabled. The facade forwards the flag through `WgpuPostProcessOptions`; `post_process_sampled.wgsl` packs it into `screen_space_flags.x` and applies a small local-contrast ambient-occlusion-style darkening before the remaining sampled post-process effects.

Native dynamic post-process labels now include the `Ssao` token when this branch is enabled, for example:

- `Neo Bloom Ssao Taa Fxaa Motion Blur Ssr Depth Of Field Tonemap Color Grading Post Process Pass`

Backend/facade standard pass coverage maps that token back to semantic `ssao` for stats/debug/capture observability.

This is not a complete SSAO implementation: it does not build or sample a depth/normal buffer, does not run a separate AO pass, does not blur AO, and does not feed AO into deferred lighting.

Validation performed for this update:

- `cargo test -p render_wgpu sampled_post_process -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

## 2026-05-19 HDR backend-visible post-process mode

`hdr` now has a backend-visible sampled post-process mode when facade `ViewQualitySettings::hdr` is enabled. The facade forwards the flag through `WgpuPostProcessOptions`; `post_process_sampled.wgsl` packs it into `screen_space_flags.y` and applies a small HDR exposure step before bloom/SSAO/other sampled effects and tonemap/gamma output.

Native dynamic post-process labels now include the `Hdr` token when this mode is enabled, for example:

- `Neo Hdr Bloom Ssao Taa Fxaa Motion Blur Ssr Depth Of Field Tonemap Color Grading Post Process Pass`

Backend/facade standard pass coverage maps that token back to semantic `hdr` when a facade graph labels HDR as a standard capability.

This is not a complete HDR pipeline: it does not yet negotiate HDR swapchain formats, expose user-facing exposure/white-point controls, validate HDR render targets, or provide display-mapping policy beyond the sampled post-process path.

Validation performed for this update:

- `cargo test -p render_wgpu sampled_post_process -- --nocapture`
- `cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`
- `cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`
- `cargo test -p engine_renderer wgpu_metrics_ -- --nocapture`
- `cargo build --bin render_smoke`
- Hidden `target\debug\render_smoke.exe` launch for 3 seconds; process did not early-exit, closed cleanly with exit code 0, and stdout/stderr were empty.

## 2026-05-19 external frame capture hook callback handoff

External frame capture hooks are no longer metadata-only when the user registers a callback. `Renderer::register_frame_capture_backend_callback` registers the same hook metadata as `register_frame_capture_backend_hook` and stores a callable hook. When a queued `RenderDoc` or `ExternalDebugger` capture reaches frame finish with `BackendHookRequested`, the renderer now invokes the callback with a `FrameCaptureHookEvent` containing backend, request id, capture label, queued frame, completed frame, dump/open flags, hook label, and hook SDK name.

Compatibility remains:

- `register_frame_capture_backend_hook` continues to register metadata-only availability for callers that only need an external handoff signal.
- `set_frame_capture_backend_available` still uses metadata-only registration.
- `unregister_frame_capture_backend_hook` clears both metadata and callback state.

This improves the external capture path from pure bookkeeping to a real user-provided callback invocation. It is still not a linked RenderDoc SDK integration: the engine does not call RenderDoc's native API itself, and SDK loading/capture-start/end remains the responsibility of the registered callback.

Validation performed for this update:

- `cargo test -p engine_renderer capture_options_validate_backend_hooks -- --nocapture`
- `cargo test -p engine_renderer capture -- --nocapture`

Resource dump test note: `frame_capture_resource_dump_counts_only_ready_resources` now accepts both delayed-destroy and same-frame reclaimed outcomes for destroyed resources, while still requiring the capture dump to mirror `FrameStats::memory` and to account for the two destroyed resources exactly once.

## 2026-05-19 external capture callback invocation observability

`FrameCapture` now exposes `external_hook_callback_invoked`, and `FrameDebugReport` mirrors it as `capture_external_hook_callback_invoked`. This distinguishes three cases:

- External hook metadata was unavailable: `external_hook_triggered=false`, `external_hook_callback_invoked=false`.
- External hook metadata was available but no callback was registered: `external_hook_triggered=true`, `external_hook_callback_invoked=false`.
- A callable external hook was registered and invoked at frame finish: `external_hook_triggered=true`, `external_hook_callback_invoked=true`.

Validation performed for this update:

- `cargo test -p engine_renderer capture -- --nocapture`
- `cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`

## 2026-05-19 matrix update: external capture callback failure observability

- Area: Frame API / frame capture / tooling observability.
- Status: `Partial` remains for native RenderDoc/external-debugger SDK integration, but the public callback handoff path now has implemented success and failure observability.
- Implemented evidence: `FrameCaptureStatus::BackendHookFailed`, `FrameCapture::external_hook_callback_failed`, `FrameCapture::external_hook_callback_failure`, `FrameDebugReport::capture_external_hook_callback_failed`, `FrameDebugReport::capture_external_hook_callback_failure`, and panic-safe callback handoff from `Renderer::register_frame_capture_backend_callback`.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer capture -- --nocapture` passed，2 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- Remaining gap: built-in RenderDoc/external debugger SDK integration is still not implemented; the engine delegates native SDK work to the registered callback and reports unsupported when no hook is available.

## 2026-05-19 matrix update: RenderGraph imported resource observability

- Area: RenderGraph / RHI resource import observability.
- Status: `Partial` remains for full resource import/export, but imported renderer texture/buffer observability is now implemented in graph stats.
- Implemented evidence: `RenderGraphStats::{imported_textures, imported_buffers, imported_texture_labels, imported_buffer_labels}` are filled by both `RenderGraphBuilder::compile()` and `RenderGraphBuilder::stats()` fallback paths.
- Propagation evidence: because `FrameStats`, `FrameDebugReport`, and `FrameCapture` carry `RenderGraphStats`, imported resource counts and labels are available through frame/debug/capture outputs without parsing builder internals.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- Remaining gap: graph resource export semantics, cross-frame transient export, and backend-wgpu graph resource import/export are still not complete.

## 2026-05-19 matrix update: RenderGraph resource export markers and stats

- Area: RenderGraph / RHI resource import/export observability.
- Status: `Partial` remains for full resource import/export, but export markers, validation, and stats observability are now implemented.
- Implemented evidence: `RenderGraphBuilder::{export_texture, export_buffer}`; `RenderGraphStats::{exported_textures, exported_buffers, exported_texture_labels, exported_buffer_labels}`; validation rejects exporting unknown graph resources.
- Propagation evidence: exported resource counts and labels are carried inside `RenderGraphStats`, therefore visible through `FrameStats.graph`, `FrameDebugReport.graph`, and `FrameCapture.graph`.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- Remaining gap: export currently marks and observes graph outputs; it does not yet create durable facade resources, preserve exported transients across frames, or bind backend-wgpu graph export/import into the surface/native backend execution path.

### 2026-05-19 matrix supplement: RenderGraph export lifetime semantics

- Exported graph resources now participate in compiled resource lifetime planning by extending `ResourceLifetime::last_pass` to the final graph pass.
- Empty graph export is rejected with `RendererError::RenderGraphValidation`.
- This improves the resource-lifetime part of the import/export gap, but does not yet create durable facade resources or backend-wgpu exported graph outputs.

## 2026-05-19 matrix update: RenderGraph compiled export list

- Area: RenderGraph / RHI resource import/export compile artifact.
- Status: `Partial` remains for full durable/backend export, but compile-time export metadata is now implemented.
- Implemented evidence: `CompiledRenderGraph::resource_exports` and `CompiledResourceExport { resource, label }` provide stable structured export records; `CompiledResourceExport` is re-exported from the crate root while remaining outside the prelude.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer prelude_keeps_graph_and_rhi_types_out_of_game_layer_imports -- --nocapture` passed，1 passed。
- Remaining gap: compiled export records do not yet materialize persistent renderer resources or backend-wgpu graph outputs.

## 2026-05-19 matrix update: RenderGraph RHI execution exports

- Area: RenderGraph / RHI resource export execution.
- Status: `Partial` remains for durable facade/backend-wgpu export, but RHI execution can now return materialized exports.
- Implemented evidence: `RhiGraphExecution`, `RhiResourceExports`, `RhiTextureExport`, `RhiBufferExport`, `RenderGraphBuilder::execute_on_rhi_with_exports()`, and `RenderGraphBuilder::execute_on_rhi_with_imports_exports()`.
- Execution evidence: `CompiledResourceExport` records are mapped to actual `RhiTexture` / `RhiBuffer` handles after command submission; tests verify imported texture/buffer exports return the same RHI handles supplied via `RhiResourceImports`.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer prelude_keeps_graph_and_rhi_types_out_of_game_layer_imports -- --nocapture` passed，1 passed。
- Remaining gap: exported RHI handles are not yet promoted to durable public renderer resources, and backend-wgpu graph/surface execution does not yet consume this export result.

### 2026-05-19 matrix supplement: RHI transient resource exports

- Export execution now covers transient graph resources, not only imported external resources.
- Test evidence: `graph_execute_on_rhi_exports_transient_resources` verifies exported transient texture/buffer outputs return materialized RHI handles and corresponding headless RHI resource counts.
- Remaining gap unchanged: these RHI handles are not yet promoted to durable renderer facade resources or backend-wgpu graph/surface outputs.

## 2026-05-19 matrix update: RenderGraph import/export stats aggregation

- Area: RenderGraph / frame stats aggregation / debug-capture observability.
- Status: implemented for aggregation of current import/export observability fields.
- Evidence: `accumulate_graph_stats()` now merges imported/exported texture/buffer counts and label vectors, so frame-level stats no longer lose resource import/export observability across multiple views/extensions.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer accumulate_graph_stats_preserves_import_export_resource_observability -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- Remaining gap: aggregation preserves observability only; durable facade resource promotion and backend-wgpu graph/surface export integration remain incomplete.

## 2026-05-19 matrix update: facade/backend graph import-export merge

- Area: RenderGraph / backend stats merge / frame observability.
- Status: implemented for current import/export stats fields.
- Evidence: `merge_facade_and_backend_graph_stats()` now accumulates imported/exported texture/buffer counts and label vectors from backend stats rather than dropping them while replacing RHI execution labels and GPU timing.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer accumulate_graph_stats_preserves_import_export_resource_observability -- --nocapture` passed，1 passed。
- Remaining gap: this prepares backend stats merging; backend-wgpu graph/surface execution still does not yet produce durable graph exports.

## 2026-05-19 matrix update: RenderGraph export label validation

- Area: RenderGraph export API / error semantics.
- Status: implemented for current export marker API.
- Evidence: graph validation rejects empty export labels and duplicate export labels across exported texture/buffer resources.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_exports_transient_resources -- --nocapture` passed，1 passed。
- Remaining gap: label validation makes export outputs stable and addressable, but durable renderer resource promotion and backend-wgpu graph/surface production remain incomplete.

## 2026-05-19 matrix update: RHI export label lookup

- Area: RenderGraph / RHI export consumption API.
- Status: implemented for current RHI export result type.
- Evidence: `RhiResourceExports::{texture_export, buffer_export, texture, buffer}` expose stable label-based lookup for materialized exported resources.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_exports_transient_resources -- --nocapture` passed，1 passed。
- Remaining gap: lookup makes RHI export consumption practical, but facade-level durable resource promotion and backend-wgpu graph/surface production are still incomplete.

## 2026-05-19 matrix update: RenderGraph extension export observability through facade

- Area: RenderGraph extension API / frame stats / debug report observability.
- Status: implemented for current export marker/stats path.
- Evidence: a public `RenderGraphExtension` can create transient texture/buffer resources, export them, and the resulting export counts and labels are visible through `FrameStats.graph` and `FrameDebugReport.graph` on the normal renderer facade frame path.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_exports_are_visible_in_frame_stats_and_debug_report -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_exports_transient_resources -- --nocapture` passed，1 passed。
- Remaining gap: facade frame stats/debug report can observe exported graph outputs, but the exported transient outputs still are not promoted into durable public `TextureHandle` / `BufferHandle` resources and backend-wgpu graph/surface execution still does not produce export handles.

### 2026-05-19 matrix supplement: RenderGraph extension exports in frame capture

- Exported graph outputs from public `RenderGraphExtension` are now verified through `FrameCapture.graph` as well as `FrameStats.graph` and `FrameDebugReport.graph`.
- Test evidence: `render_graph_extension_exports_are_visible_in_frame_stats_and_debug_report` queues an internal capture and asserts exported texture/buffer counts and labels survive into the capture payload.

## 2026-05-19 matrix update: profiled facade RenderGraph exports

- Area: RenderGraph extension exports / GPU profiler RHI path / frame observability.
- Status: implemented for export observability on the profiled headless-RHI facade path.
- Evidence: `profiled_render_graph_extension_exports_remain_visible_in_frame_outputs` enables `RendererConfig::gpu_profiling`, exports a transient texture from a public graph extension, and verifies `FrameStats.graph`, `FrameDebugReport.graph`, and `FrameCapture.graph` preserve the export label while GPU time is reported.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer profiled_render_graph_extension_exports_remain_visible_in_frame_outputs -- --nocapture` passed，1 passed。
- Remaining gap: profiled facade/RHI observability is closed for current export stats, but durable public resource promotion and backend-wgpu surface export handles remain incomplete.

### 2026-05-19 matrix supplement: graph/export regression suite

- Focused graph regression after this set of import/export changes: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_ -- --nocapture` passed，38 passed。
- This covers headless graph validation, RHI execution, facade graph extensions, import/export observability, and wgpu graph execution smoke paths.

## 2026-05-19 matrix update: RenderGraph resource label summaries

- Area: RenderGraph import/export consumption API.
- Status: implemented for current stats-level resource labels.
- Evidence: `RenderGraphStats::{imported_resource_labels, exported_resource_labels, has_resource_imports, has_resource_exports}` and `RenderGraphResourceLabels` provide structured public consumption of import/export label data through `FrameStats.graph`, `FrameDebugReport.graph`, and `FrameCapture.graph`.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_exports_are_visible_in_frame_stats_and_debug_report -- --nocapture` passed，1 passed。
- Remaining gap: this improves public consumption of export observability, but durable public renderer resource promotion and backend-wgpu graph/surface export production are still incomplete.

## 2026-05-19 matrix update: RenderGraph export validation through facade

- Area: RenderGraph extension API / facade error path.
- Status: implemented for duplicate export-label validation on the public frame path.
- Evidence: `render_graph_extension_rejects_duplicate_export_labels_through_facade` registers a public graph extension with duplicate texture/buffer export labels and observes `RendererError::RenderGraphValidation` through `render_view()`/`finish()`.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_rejects_duplicate_export_labels_through_facade -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed。
- Remaining gap: validation and observability are closed for current export metadata, but durable resource promotion and backend-wgpu graph/surface export production remain incomplete.

## 2026-05-19 matrix update: wgpu RHI RenderGraph exports

- Area: backend-wgpu / RenderGraph RHI export execution.
- Status: implemented for direct wgpu RHI graph execution exports; `Partial` remains for surface/standard-frame backend-wgpu export integration and durable facade resource promotion.
- Evidence: `graph_execute_on_wgpu_exports_transient_resources` executes a graph with exported transient texture/buffer resources on `WgpuRhiDevice` and verifies label lookup in `RhiResourceExports`.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_wgpu_exports_transient_resources -- --nocapture` passed，1 passed。
- Remaining gap: direct wgpu RHI graph export works, but the main backend-wgpu surface renderer path still does not surface graph exports as persistent public resources.

## 2026-05-19 matrix update: pipeline cache backend-object coverage helpers

- Area: Pipeline / pipeline key / cache observability.
- Status: `Partial` remains for complete backend-native pipeline cache coverage, but public coverage helpers are implemented.
- Evidence: `PipelineCacheStats::{ready_backend_object_gap, used_backend_object_gap, all_ready_entries_have_backend_objects, all_used_entries_have_backend_objects, has_complete_facade_backend_object_coverage}` make facade/backend object coverage explicit.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer pipeline_warmup_validates_pipeline_keys -- --nocapture` passed，1 passed。
- Remaining gap: facade cache entries can still be ready without a backend object; backend-wgpu complete cache coverage and promotion of all facade entries to native objects remain incomplete.

## 2026-05-19 matrix update: pipeline cache merge coverage semantics

- Area: Pipeline cache / backend inventory merge / coverage observability.
- Status: implemented for current public stats semantics.
- Evidence: `pipeline_cache_stats_merge_preserves_facade_counts_and_backend_inventory` now asserts backend object inventory does not clear facade backend-object gap helpers unless the facade gap counters are zero.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer pipeline_cache_stats_merge_preserves_facade_counts_and_backend_inventory -- --nocapture` passed，1 passed。
- Remaining gap: complete backend-native pipeline cache coverage is still incomplete; this change prevents observability from overstating completion.

## 2026-05-19 matrix update: backend-wgpu pipeline cache coverage merge semantics

- Area: backend-wgpu pipeline cache stats merge / complete backend cache observability.
- Status: implemented for current stats merge semantics; complete backend-native cache coverage remains `Partial`.
- Evidence: `wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory` now verifies native backend object inventory does not incorrectly clear facade backend-object gap helpers.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory -- --nocapture` passed，1 passed。
- Remaining gap: complete backend-native pipeline cache coverage is still not implemented for every facade cache entry.

## 2026-05-19 matrix update: pipeline cache coverage helpers in debug/capture outputs

- Area: Pipeline cache / editor-debug-capture observability.
- Status: implemented for current helper coverage path.
- Evidence: `frame_debug_report_summarizes_last_frame_for_editor` now asserts `FrameDebugReport.pipeline_cache` and `FrameCapture.pipeline_cache` preserve `PipelineCacheStats` helper semantics for backend-object coverage gaps.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- Remaining gap: helper observability is complete, but actual complete backend-native pipeline cache coverage remains partial.

## 2026-05-19 - Pipeline cache coverage helpers in debug/capture payloads

- Added public `PipelineCacheStats` helpers that report ready/used facade entries without backend pipeline objects and aggregate complete-coverage booleans.
- Covered propagation through `FrameDebugReport` and `FrameCapture`, allowing editor tooling to report incomplete backend-object residency separately from facade-level warmup state.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed: 1 test.

## 2026-05-19 - Public RenderGraph export promotion

- Closed the explicit public-facade RenderGraph export promotion gap for RHI/headless execution: `Renderer::execute_graph_to_resources` now runs a graph, captures `RhiResourceExports`, reads back exported transient texture/buffer data, and materializes durable public `TextureHandle` / `BufferHandle` resources.
- Added `RendererGraphExecution` / `RendererGraphResourceExports` lookup helpers so tools can resolve export labels directly to public handles. Exported imported resources map back to the original renderer handle with `promoted: false`; transient exports create new ready resources with `promoted: true`.
- Status impact: RenderGraph export is no longer label-only for explicit public graph execution. Remaining gap: backend-wgpu surface/standard-frame graph export integration still needs native frame-output handle production before the complete renderer goal can close.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_exported_transients_to_public_handles -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_ -- --nocapture` passed: 3 tests.

## 2026-05-19 - Public graph import data upload and exported-import writeback

- Strengthened the explicit public RenderGraph execution path: imported renderer buffers/textures are now uploaded into the RHI import resources before callbacks execute.
- Exported imported renderer resources now write RHI results back into their original public `BufferHandle` / `TextureHandle`, preserving durable handle identity while updating observable public bytes.
- Status impact: public graph resource import/export is no longer handle-only for the covered RHI/headless path; data flow is observable through `buffer_bytes`, `texture_bytes`, and export `promoted` flags.
- Remaining gap: imported texture upload/writeback is currently limited to single-sample 2D base-level supported readback formats, and backend-wgpu surface/standard-frame graph export integration remains open.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_uploads_and_writes_back_imported_exports -- --nocapture` passed: 1 test. `execute_graph_to_resources_promotes_exported_transients_to_public_handles` passed: 1 test. `graph_execute_on_rhi_` passed: 3 tests.

## 2026-05-19 - Depth graph import/export writeback

- Added RHI `write_texture_depth32f` and connected it to public graph import/export execution.
- Public `Renderer::execute_graph_to_resources` now uploads imported `Depth32Float` public texture bytes into RHI, lets graph callbacks update them, and writes exported imported depth results back to the same public handle.
- Status impact: public graph texture import/export data flow now covers color readback formats plus `Depth32Float` for the supported single-sample 2D base-level path.
- Remaining gap: array/cube/3D/mip/MSAA texture import/writeback and backend-wgpu surface/standard-frame graph export integration remain incomplete.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_uploads_and_writes_back_depth_imports -- --nocapture` passed: 1 test. `execute_graph_to_resources_` passed: 3 tests. `graph_execute_on_rhi_` passed: 3 tests.

## 2026-05-19 - 8-bit sRGB/BGRA graph export promotion

- Expanded RHI raw 8-bit color texture read/write support from only `Rgba8Unorm` to `Rgba8UnormSrgb` and `Bgra8UnormSrgb`.
- Public graph export promotion now materializes transient sRGB/BGRA graph textures as durable public `TextureHandle` resources while preserving `TextureInfo::format` and public `texture_bytes` contents.
- Status impact: the explicit public RHI/headless graph export path now covers all current 8-bit color texture formats plus previous float/depth paths for supported single-sample 2D base-level resources.
- Remaining gap: array/cube/3D/mip/MSAA texture import/writeback and backend-wgpu surface/standard-frame graph export integration remain incomplete.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_8bit_srgb_and_bgra_exports -- --nocapture` passed: 1 test. `execute_graph_to_resources_` passed: 4 tests. `graph_execute_on_rhi_` passed: 3 tests.

## 2026-05-19 - Public graph unsupported texture-shape gate

- Added explicit validation for unsupported public graph imported texture shapes. Mipped, array, and MSAA imported textures now return `RenderGraphValidation` through `Renderer::execute_graph_to_resources` rather than producing ambiguous upload/writeback behavior.
- Status impact: public graph texture import/export has a clearer support boundary: supported single-sample 2D base-level paths execute and write back; unsupported shape classes have tests and user-visible errors.
- Remaining gap: these unsupported texture classes are not implemented yet; backend-wgpu surface/standard-frame graph export integration also remains incomplete.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_rejects_unsupported_imported_texture_shapes -- --nocapture` passed: 1 test. `execute_graph_to_resources_` passed: 5 tests.

## 2026-05-19 - Public graph export query surface

- Added `Renderer::last_graph_execution` and `Renderer::last_graph_resource_exports` so promoted public graph export handles remain queryable through the renderer facade after `execute_graph_to_resources` returns.
- Failed public graph execution clears the cached graph execution result, preventing stale export handles from being reported to tools.
- Status impact: explicit public graph export promotion now has a facade-level observability/query surface in addition to the immediate return value.
- Remaining gap: frame/capture/debug standard-frame graph export integration and backend-wgpu surface graph exported outputs remain incomplete.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture` passed: 1 test. `execute_graph_to_resources_` passed: 5 tests.

## 2026-05-19 - Frame debug/capture explicit graph export observability

- Added `FrameDebugReport::public_graph_execution` and `FrameCapture::public_graph_execution` so promoted explicit public graph export handles are visible from frame debug and capture payloads.
- Covered successful buffer export observability through debug report and internal capture, plus existing stale-result clearing on failed graph execution.
- Status impact: explicit public graph export promotion now has immediate return-value, renderer query, frame-debug, and capture observability surfaces.
- Remaining gap: this observes explicit `execute_graph_to_resources` exports; backend-wgpu surface/standard-frame graph output production as durable public resources remains incomplete.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture` passed: 1 test. `frame_debug_report_` passed: 3 tests. `execute_graph_to_resources_` passed: 5 tests.

## 2026-05-19 - FrameStats explicit graph export observability

- Added `FrameStats::public_graph_execution` so explicit public graph export handles are observable directly from frame output, not only through `FrameDebugReport` or `FrameCapture`.
- Capture payloads now mirror this field from `FrameStats`, keeping stats/debug/capture graph-export observability consistent.
- Status impact: explicit public graph export promotion now has immediate return-value, renderer query, frame stats, frame debug, and capture observability surfaces.
- Remaining gap: backend-wgpu surface/standard-frame graph export production as durable public frame outputs remains incomplete.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture` passed: 1 test. `frame_debug_report_` passed: 3 tests. `execute_graph_to_resources_` passed: 5 tests.

## 2026-05-19 - Capture resource dump explicit graph export counts

- Added `FrameCaptureResourceDump` counters for explicit public graph exports: exported textures/buffers, promoted texture/buffer exports, and imported texture/buffer exports.
- Covered a mixed graph export case with one promoted transient texture and one exported imported buffer, verifying dump counts and public buffer writeback.
- Status impact: capture resource dumps now summarize explicit graph export resource impact while detailed handles remain queryable through `public_graph_execution`.
- Remaining gap: backend-wgpu surface/standard-frame graph export production as durable public frame outputs remains incomplete.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_capture_resource_dump_counts_public_graph_exports -- --nocapture` passed: 1 test. `resource_dump` passed: 2 tests. `execute_graph_to_resources_` passed: 5 tests.

## 2026-05-19 - Public graph export handle lifetime cleanup

- `Renderer::destroy` now clears the latest explicit public graph execution when the destroyed texture/buffer handle is referenced by that graph's exports.
- This prevents `last_graph_execution`, frame stats, debug reports, capture payloads, and resource dump graph-export counters from retaining stale handles after public resource destruction.
- Updated the memory stats resident/destroy regression to current submission-boundary semantics: submitted frames report `ResourceReclaimPolicy::SubmissionBoundary` and reclaimed resource counts.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer destroying_public_graph_export_handles_clears_last_graph_execution -- --nocapture` passed: 1 test. `frame_stats_report_resident_memory_and_delayed_destroy_count` passed: 1 test. `destroy` passed: 13 tests. `execute_graph_to_resources_` passed: 5 tests.

## 2026-05-19 - Durable public frame outputs for non-surface targets

- Added `FramePublicFrameOutput` / `FramePublicOutputSource` and exposed `public_frame_outputs` through `FrameStats`, `FrameDebugReport`, and `FrameCapture`.
- Headless views now produce durable public output textures. Existing texture, texture-view, and external render-target frame targets are reported as existing public output handles.
- Capture resource dump expectations now include the generated headless public output texture in ready texture counts.
- Status impact: non-surface frame output has a public durable handle and stats/debug/capture observability. This does not claim backend-wgpu surface export completion.
- Remaining gap: backend-wgpu surface/standard-frame graph export production as durable public frame outputs remains incomplete.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_are_durable_public_textures -- --nocapture` passed: 1 test. `resource_dump` passed: 2 tests. `frame_debug_report_` passed: 3 tests. `frame_builds_stats_from_scene_and_view` passed: 1 test.

## 2026-05-19 - Headless public frame output clear data

- Headless-generated public frame output textures now encode camera clear color into their public bytes, making the durable headless output a real clear-output artifact rather than a zero-filled placeholder.
- Added regression coverage for non-black clear color byte output on the generated public texture, while preserving stats/debug/capture observability.
- Status impact: non-surface headless frame output is closer to real renderer output semantics. Surface/swapchain output is still not exposed as a durable public resource.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_are_durable_public_textures -- --nocapture` passed: 1 test. `resource_dump` passed: 2 tests. `frame_builds_stats_from_scene_and_view` passed: 1 test.

## 2026-05-19 - Public texture-target frame clear writeback

- Direct public texture frame targets now receive camera clear-color bytes when reported as `FramePublicOutputSource::ExistingTargetTexture`.
- External render-target color textures use the same clear writeback path and remain visible as existing public output handles.
- Status impact: non-surface public frame output now covers generated headless outputs plus direct/external public texture-backed outputs with real clear data. TextureView subresource writeback and surface/swapchain export remain incomplete.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer texture_frame_outputs_write_clear_color_to_existing_public_texture -- --nocapture` passed: 1 test. `frame_outputs` passed: 3 tests. `resource_dump` passed: 2 tests.

## 2026-05-19 public frame output writeback slice

Implemented:

- Headless public frame outputs are durable public textures populated from camera clear color.
- `RenderTarget::Texture` frame outputs now expose the existing public texture handle and write clear color bytes into it.
- External render-target frame outputs now expose the existing public color texture handle and write clear color bytes into it.
- Frame stats, debug reports, captures, and resource dumps account for public frame outputs and public graph exports.

Still incomplete for the full renderer-layer goal:

- `RenderTarget::TextureView` now reports the owner handle and adjusted extent, and writes clear-color bytes into the selected single-mip, 2D-compatible mip/layer subresource.
- Main surface and swapchain outputs still do not provide a durable public output texture.
- This slice writes clear-color output only; it is not yet a full shaded frame readback path.

Targeted validation passed:

- `cargo test -p engine_renderer texture_frame_outputs_write_clear_color_to_existing_public_texture -- --nocapture` passed 1 test.
- `cargo test -p engine_renderer frame_outputs -- --nocapture` passed 3 tests.
- `cargo test -p engine_renderer resource_dump -- --nocapture` passed 2 tests.


## 2026-05-19 TextureView public frame output writeback slice

Implemented:

- `RenderTarget::TextureView` public frame outputs now keep the existing owner `TextureHandle` and expose the base-mip width/height.
- Texture-view outputs write camera clear color bytes into the selected `base_mip`, `base_layer`, and `layer_count` range for 2D-compatible single-mip views.
- The writeback updates texture backing bytes, layout metadata, revision, and ready status, matching direct texture/external target public output behavior.

Still incomplete for the full renderer-layer goal:

- Frame output writeback still represents clear-color output, not a complete shaded frame readback.
- Multi-mip view rendering/readback and durable surface/swapchain output remain incomplete.

Targeted validation passed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer texture_view_frame_outputs_write_clear_color_to_target_subresource -- --nocapture` passed 1 test.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture` passed 4 tests.

## 2026-05-19 public frame output subresource metadata slice

Implemented:

- `FramePublicFrameOutput` now carries `base_mip`, `mip_count`, `base_layer`, and `layer_count`.
- Headless, direct texture, and external target outputs report the default base subresource.
- TextureView outputs report their real mip/layer range, matching the texture-view clear-color writeback path.
- Public frame output metadata is propagated through frame stats, debug report, and capture because those surfaces store the same `FramePublicFrameOutput` payload.

Still incomplete for the full renderer-layer goal:

- Subresource metadata improves observability but does not replace shaded-frame readback.
- Multi-mip render-target views are still intentionally rejected by render-target validation.
- Surface/swapchain outputs still do not expose durable public frame output textures.

Targeted validation passed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer texture_view_frame_outputs_write_clear_color_to_target_subresource -- --nocapture` passed 1 test.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture` passed 4 tests.

## 2026-05-19 public frame output scene/material preview slice

Implemented:

- Public frame output bytes now depend on view scene/material state when visible geometry exists.
- The preview path uses actual visibility/layer checks, LOD-selected object resources, standard material `base_color`, optional `base_color_texture` average color, and emissive contribution.
- Empty public frame outputs retain camera clear-color behavior.
- The same output path applies to headless-generated outputs and existing public texture/texture-view/external target writeback.

Still incomplete for the full renderer-layer goal:

- This is not a full rasterized shaded-frame readback.
- Lighting, shadows, depth-tested coverage, post-process, backend-wgpu swapchain readback, and per-pixel geometry rasterization remain incomplete.
- Surface/swapchain outputs still do not expose durable public frame output textures.

Targeted validation passed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_use_base_color_texture_average -- --nocapture` passed 1 test.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture` passed 5 tests.

## 2026-05-19 public frame output content provenance slice

Implemented:

- `FramePublicFrameOutput` now exposes `content`, `visible_geometry`, and `material_samples`.
- `FramePublicOutputContent::ClearColor` identifies empty clear-only output.
- `FramePublicOutputContent::SceneMaterialPreview` identifies output bytes derived from actual visible scene/material state.
- The provenance fields travel through frame stats, debug report, and capture because those surfaces share the same public frame output payload.

Still incomplete for the full renderer-layer goal:

- Provenance metadata explains the preview output path, but does not make it a full shaded-frame readback.
- Surface/swapchain durable public output and backend-wgpu per-pixel readback remain incomplete.

Targeted validation passed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_use_base_color_texture_average -- --nocapture` passed 1 test.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture` passed 5 tests.

## 2026-05-19 public frame output lighting/environment preview slice

Implemented:

- Scene/material public frame output preview now includes view-layer-matched light tint.
- Environment diffuse/background contribution is blended into the preview when a scene environment is bound.
- Manual exposure scales the preview color.
- `FramePublicFrameOutput` now exposes `light_samples` and `environment_samples` for stats/debug/capture observability.

Still incomplete for the full renderer-layer goal:

- This is not physically based lighting, shadowed rasterization, post-process readback, or backend-wgpu surface readback.
- The preview path is deterministic facade/headless observability, not a replacement for full shaded frame output.

Targeted validation passed:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_include_light_environment_and_exposure_preview -- --nocapture` passed 1 test.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture` passed 6 tests.

## Coverage update - public frame output post-process preview (2026-05-19 18:00:29 +08:00)

Status: partial renderer-layer coverage improved.

Implemented:
- FramePublicFrameOutput::post_process_samples records deterministic preview contribution count.
- Public frame output bytes now reflect render-path tonemapping and selected quality/post-process settings in the scene-material preview path.
- Existing clear-only outputs keep post_process_samples = 0.

Validation:
- cargo test -p engine_renderer frame_outputs -- --nocapture passed: 7 passed, 0 failed.

Remaining gap:
- This is still CPU preview observability. Full renderer completion still requires real backend post-process execution/readback coverage across the documented renderer API surface.

## 2026-05-19 - Public graph mipped D2 base-mip import/export

Area: RenderGraph public resource import/export.

Status: partial renderer-layer coverage improved.

Implemented:
- Explicit Renderer::execute_graph_to_resources imports no longer reject public TextureDimension::D2 textures solely because mip_levels > 1.
- The current RHI path uploads the public texture's base mip, lets graph passes read/write it, and writes the base-mip bytes back to the original public TextureHandle export.
- Array-shaped imports and resolved MSAA D2 imports now have compatibility paths; native sample-level MSAA graph textures remain explicitly outside the current RHI model instead of being treated as fully supported.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 6 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture passed: 1 passed, 0 failed.

Remaining gap:
- This is base-mip compatibility only. True multi-mip subresource execution/readback, array/cube/3D texture imports, MSAA imports, and backend-wgpu surface/standard-frame graph export integration remain incomplete.

## 2026-05-19 - Public graph generated mip base upload

Area: RenderGraph public resource import/export / generated texture mips.

Status: partial renderer-layer coverage improved.

Implemented:
- Explicit graph imports can upload the base mip from a generated D2 mip chain when StoredTexture::layout is absent because generate_mips() compacted the texture into mip-chain bytes.
- Export writeback updates the original public texture's base payload and clears mips_generated, preventing stale generated mips from being reported as valid after graph mutation.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 7 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture passed: 1 passed, 0 failed.

Remaining gap:
- This does not expose per-mip graph subresource reads/writes. True multi-mip, array/cube/3D, MSAA, and backend-wgpu surface graph output integration remain incomplete.

## 2026-05-19 - Headless/stub surface public frame outputs

Area: Frame output observability / surface targets.

Status: partial renderer-layer coverage improved.

Implemented:
- FramePublicOutputSource now distinguishes HeadlessMainSurfaceGenerated and HeadlessSurfaceGenerated outputs.
- Headless RenderTarget::MainSurface and valid stub RenderTarget::Surface(handle) produce durable public output textures with extent, format, subresource metadata, and preview provenance.
- Backend-owned wgpu surfaces now produce public output only when the backend surface supports COPY_SRC readback and a completed wgpu frame has readback bytes; unsupported/no-readback surfaces still return no fabricated output.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture passed: 8 passed, 0 failed.

Remaining gap:
- Backend-wgpu surface readback is no longer entirely missing, but this older slice only closed headless/stub surface observability. The backend-wgpu path is tracked by the later surface-readback entry and remains partial until async/optional readback, unsupported-reason observability, and graph-export integration are complete.

## 2026-05-19 - Public frame output multi-mip texture-view preview

Area: Frame output observability / texture-view render targets.

Status: partial renderer-layer coverage improved.

Implemented:
- RenderTarget::TextureView validation now accepts non-zero mip ranges that fit inside the target texture.
- Public frame output writeback packs preview bytes for each selected mip in order while preserving source, content, mip, layer, and preview provenance metadata through FramePublicFrameOutput.
- Existing single-mip texture-view writeback remains layout-addressable and covered by the previous subresource test.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture passed: 9 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer texture_view_render_targets_validate_subresource_ranges -- --nocapture passed: 1 passed, 0 failed.

Remaining gap:
- Multi-mip output bytes are packed preview/readback bytes, not full RHI multi-subresource upload/readback metadata. True mip/layer subresource RHI IO, array/cube/3D/MSAA graph IO, and backend-wgpu surface output integration remain incomplete.

## 2026-05-19 - Public frame output subresource byte layout

Area: Frame output observability / capture/debug payloads.

Status: partial renderer-layer coverage improved.

Implemented:
- Added FramePublicFrameOutputSubresource and FramePublicFrameOutput::subresources.
- Public frame output stats/debug/capture payloads now describe byte offsets and byte lengths for single-subresource and packed multi-mip texture-view output payloads.
- Texture-view tests now verify single-mip layer ranges and multi-mip packed offsets.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture passed: 9 passed, 0 failed.

Remaining gap:
- This closes public payload layout observability only. True RHI mip/layer subresource IO, array/cube/3D/MSAA graph IO, and complete backend-wgpu surface output/readback remain incomplete.

## 2026-05-19 - Public graph texture export descriptor metadata

Area: RenderGraph public resource export observability.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphTextureExport now includes dimension, extent/layer count, mip count, sample count, format, and usage metadata.
- Metadata is populated for promoted transient graph texture exports and imported public texture exports.
- Existing last-graph, frame-debug, and capture graph-export observability continue to preserve the expanded export payload.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 7 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture passed: 1 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture passed: 1 passed, 0 failed.

Remaining gap:
- This closes export descriptor observability for explicit public graph execution. Backend-wgpu surface/standard-frame graph export production, true subresource RHI IO, and array/cube/3D/MSAA graph IO remain incomplete.

## 2026-05-19 - Public graph buffer export descriptor metadata

Area: RenderGraph public buffer export observability.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphBufferExport now includes size and usage metadata.
- Metadata is populated for promoted transient graph buffer exports and imported public buffer exports.
- Existing last-graph, frame-debug, and capture graph-export observability preserve the expanded buffer export payload.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 7 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture passed: 1 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture passed: 1 passed, 0 failed.

Remaining gap:
- This closes explicit public graph buffer export metadata. Backend-wgpu surface/standard-frame graph export production and broader backend-native resource export integration remain incomplete.

## 2026-05-19 - Public graph texture export represented subresource layout

Area: RenderGraph public texture export observability.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphTextureExport now includes subresources describing represented export byte layout.
- Promoted transient texture exports expose base-mip byte layout directly in the export payload.
- Imported mipped D2 texture exports keep descriptor mip_levels while subresources truthfully reports only the represented base mip.
- Existing last-graph, frame-debug, and capture graph-export observability preserve the expanded texture export payload.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 7 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture passed: 1 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture passed: 1 passed, 0 failed.

Remaining gap:
- This closes represented-byte layout observability for explicit public graph texture exports. It does not implement true multi-mip/layer RHI IO, array/cube/3D/MSAA graph IO, or backend-wgpu surface/standard-frame graph export production.

## 2026-05-19 - Public graph buffer export represented byte range

Area: RenderGraph public buffer export observability.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphBufferExport now includes byte_offset and byte_len.
- Promoted transient buffer exports and imported public buffer exports report full-buffer represented ranges.
- Existing last-graph, frame-debug, and capture graph-export observability preserve the expanded buffer export payload.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 7 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture passed: 1 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture passed: 1 passed, 0 failed.

Remaining gap:
- This closes explicit public graph buffer represented-range observability. Backend-wgpu surface/standard-frame graph export production and broader backend-native resource export integration remain incomplete.

## 2026-05-19 - Public graph export source provenance

Area: RenderGraph public export observability.

Status: partial renderer-layer coverage improved.

Implemented:
- Added RendererGraphExportSource with PromotedTransient, ImportedPublic, BackendMainSurfaceReadback, and BackendSurfaceReadback variants.
- RendererGraphTextureExport and RendererGraphBufferExport now expose source provenance while retaining promoted for compatibility.
- Promoted transient exports and imported public writeback exports are covered for both texture and buffer paths.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 7 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture passed: 1 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture passed: 1 passed, 0 failed.

Remaining gap:
- This improves explicit graph export provenance only. Backend-wgpu surface/standard-frame graph export production and broader backend-native resource export integration remain incomplete.

## 2026-05-19 - Public graph texture export subresource coverage flags

Area: RenderGraph public texture export observability.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphTextureExport now includes complete_mip_coverage, complete_layer_coverage, and complete_subresource_coverage.
- Single-mip D2 promoted/imported exports report full coverage.
- Mipped D2 imported base-mip writeback reports partial coverage, making the descriptor-vs-represented-byte distinction explicit.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 7 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture passed: 1 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture passed: 1 passed, 0 failed.

Remaining gap:
- These flags make partial explicit graph texture export coverage observable. They do not implement true mip/layer RHI IO, array/cube/3D/MSAA graph IO, or backend-wgpu surface/standard-frame graph export production.

## 2026-05-19 - Public graph imported texture subregion upload

Area: RenderGraph public texture import/upload/writeback.

Status: partial renderer-layer coverage improved.

Implemented:
- Explicit public graph texture imports now have focused coverage for TextureUpdate subregion layouts.
- The RHI import path uploads partial public texture payloads at their declared x/y offset before graph execution.
- Export writeback replaces the public texture with full base-mip readback bytes and reports full represented base-mip coverage in the export payload.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 8 passed, 0 failed.

Remaining gap:
- This covers 2D single-sample base-mip subregion upload into explicit graph execution. It does not implement arbitrary graph subregion export, true mip/layer RHI IO, array/cube/3D/MSAA graph IO, or backend-wgpu surface/standard-frame graph export production.

## 2026-05-19 - Public graph imported buffer subrange update coverage

Area: RenderGraph public buffer import/upload/writeback.

Status: partial renderer-layer coverage improved.

Implemented:
- Added focused coverage for BufferUpdate at a non-zero offset before explicit graph import.
- Explicit graph import observes the merged full public buffer payload.
- Export writeback replaces the public buffer with full exported bytes and reports full-buffer represented byte range metadata.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 9 passed, 0 failed.

Remaining gap:
- This validates buffer subrange update semantics before graph import, but does not implement minimal-range upload scheduling, backend-wgpu standard-frame graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public graph buffer export byte coverage flag

Area: RenderGraph public buffer export observability.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphBufferExport now includes complete_byte_coverage.
- Promoted transient buffer exports, imported public buffer exports, and subrange-updated public buffer exports all report full-buffer coverage for current explicit graph semantics.
- Existing last-graph, frame-debug, and capture graph-export observability preserve the expanded buffer export payload.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 9 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture passed: 1 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture passed: 1 passed, 0 failed.

Remaining gap:
- This closes explicit graph full-buffer coverage observability. It does not implement partial buffer export ranges, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public graph export aggregate coverage helpers

Area: RenderGraph public export observability / tooling helpers.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphResourceExports now summarizes incomplete texture subresource and buffer byte coverage.
- Full promoted/imported texture and buffer exports report all-complete aggregate status.
- Mipped D2 base-mip texture writeback reports incomplete texture subresource coverage through the aggregate helpers.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 9 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture passed: 1 passed, 0 failed.
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture passed: 1 passed, 0 failed.

Remaining gap:
- These helpers summarize explicit graph export coverage only. They do not implement missing RHI mip/layer IO, partial buffer export ranges, array/cube/3D/MSAA graph IO, or backend-wgpu standard-frame/surface graph exports.

## 2026-05-19 - Public frame output aggregate subresource helpers

Area: Frame output observability / tooling helpers.

Status: partial renderer-layer coverage improved.

Implemented:
- FramePublicFrameOutput now reports represented subresource byte length, packed-subresource status, and complete view-layout status through helper methods.
- FrameStats now aggregates packed public output count, represented public output bytes, and complete-layout status for all public outputs.
- Single-mip texture-view output and multi-mip packed texture-view output both have focused coverage.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture passed: 9 passed, 0 failed.

Remaining gap:
- These helpers summarize current public frame output payload layout only. They do not implement true backend/swapchain readback, RHI mip/layer subresource IO, or full rasterized shaded-frame output.

## 2026-05-19 - Public graph imported D1 texture base-mip execution

Area: RenderGraph public texture import/export shape support.

Status: partial renderer-layer coverage improved.

Implemented:
- Explicit public graph texture import/writeback now accepts single-sample D1 base-mip textures in addition to D2.
- D1 textures are executed through the current height-1 RHI texture path and exported back to the original public texture handle.
- Export metadata preserves TextureDimension::D1 and reports represented base-mip byte layout plus complete coverage flags.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 10 passed, 0 failed.

Remaining gap:
- This closes D1/D2 base-mip explicit public graph texture execution only. Array/cube/3D/MSAA imports, true mip/layer RHI IO, backend-wgpu standard-frame/surface graph exports, and broader backend-native resource export integration remain incomplete.

## 2026-05-19 - Public graph imported D2Array flattened base-mip execution

Area: RenderGraph public texture import/export shape support.

Status: partial renderer-layer coverage improved.

Implemented:
- Explicit public graph texture import/writeback now accepts single-sample D2Array base-mip textures.
- D2Array layers are flattened as height-stacked rows for the current RHI-compatible execution path.
- Export metadata preserves TextureDimension::D2Array and reports represented layer coverage and complete subresource coverage for the base mip.
- Unsupported shape coverage now verifies Cube and MSAA imports still reject explicitly.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 11 passed, 0 failed.

Remaining gap:
- This closes D1/D2/D2Array base-mip explicit public graph texture execution only. It is not true layer-aware RHI/backend execution. Cube/cube-array/3D/MSAA imports, arbitrary mip/layer RHI IO, backend-wgpu standard-frame/surface graph exports, and broader backend-native resource export integration remain incomplete.

## 2026-05-19 - Public graph imported Cube/CubeArray flattened base-mip execution

Area: RenderGraph public texture import/export shape support.

Status: partial renderer-layer coverage improved.

Implemented:
- Explicit public graph texture import/writeback now accepts single-sample Cube and CubeArray base-mip textures through the same flattened layer-stack path used by D2Array.
- Cube faces/layers are flattened as height-stacked rows for current RHI-compatible execution.
- Export metadata preserves TextureDimension::Cube/CubeArray and reports represented face/layer coverage and complete subresource coverage for the base mip.
- Unsupported shape coverage now verifies D3 and MSAA imports still reject explicitly.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 12 passed, 0 failed.

Remaining gap:
- This closes D1/D2/D2Array/Cube/CubeArray base-mip explicit public graph texture execution only. It is not true cube/layer-aware RHI/backend execution. 3D/MSAA imports, arbitrary mip/layer RHI IO, backend-wgpu standard-frame/surface graph exports, and broader backend-native resource export integration remain incomplete.

## 2026-05-19 - Public graph imported D3 flattened base-mip execution

Area: RenderGraph public texture import/export shape support.

Status: partial renderer-layer coverage improved.

Implemented:
- Explicit public graph texture import/writeback now accepts single-sample D3 base-mip textures through the flattened depth-stack path.
- D3 depth slices are flattened as height-stacked rows for current RHI-compatible execution.
- Export metadata preserves TextureDimension::D3 and reports represented depth coverage and complete subresource coverage for the base mip.
- Unsupported shape coverage now verifies MSAA imports still reject explicitly.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 13 passed, 0 failed.

Remaining gap:
- This closes D1/D2/D2Array/Cube/CubeArray/D3 base-mip explicit public graph texture execution only. It is not true volume/layer/cube-aware RHI/backend execution. MSAA imports, arbitrary mip/layer/depth RHI IO, backend-wgpu standard-frame/surface graph exports, and broader backend-native resource export integration remain incomplete.

## 2026-05-19 - Public graph imported Depth32Float D2Array flattened execution

Area: RenderGraph public depth texture import/export shape support.

Status: partial renderer-layer coverage improved.

Implemented:
- Added focused coverage for TextureFormat::Depth32Float D2Array imports through the flattened layer-stack explicit graph path.
- RHI depth32f read/write functions observe the flattened depth array values before export writeback.
- Export metadata preserves D2Array/depth descriptor information and complete represented subresource coverage.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 14 passed, 0 failed.

Remaining gap:
- This validates flattened depth-array compatibility only. It does not implement native depth-array layer addressing, MSAA imports, arbitrary mip/layer/depth RHI IO, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public graph imported non-base mip represented execution

Area: RenderGraph public texture import/export mip support.

Status: partial renderer-layer coverage improved.

Implemented:
- Explicit public graph texture import/writeback now accepts a complete non-base mip payload from TextureUpdate.
- RHI execution uses the represented mip extent instead of the base texture extent for that import.
- Export metadata reports the represented mip level and partial mip/subresource coverage.
- Base subregion, generated base mip, flattened array/cube/D3, depth array, and D1/D2 paths remain covered by the same regression filter.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 15 passed, 0 failed.

Remaining gap:
- This supports one complete represented mip at a time. It does not implement simultaneous multi-mip graph IO, partial non-base mip subregions, native mip/layer/depth RHI addressing, MSAA imports, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public graph imported non-base mip subregion execution

Area: RenderGraph public texture import/export mip/subregion support.

Status: partial renderer-layer coverage improved.

Implemented:
- Explicit public graph texture import/writeback now accepts x/y subregion payloads inside a complete represented non-base mip layer/depth range.
- RHI execution uses the represented full mip extent and uploads the subregion at the declared offset.
- Export writeback replaces the public texture payload with the represented full mip bytes and reports partial mip/subresource coverage.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 16 passed, 0 failed.

Remaining gap:
- This supports one represented mip at a time. It does not implement simultaneous multi-mip graph IO, native mip/layer/depth RHI addressing, partial layer/depth coverage, MSAA imports, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public graph texture import support query

Area: RenderGraph public texture import capability observability.

Status: partial renderer-layer coverage improved.

Implemented:
- Added Renderer::graph_texture_import_support and RendererGraphTextureImportSupport.
- The query reports supported/unsupported status, reason, descriptor metadata, flattened compatibility, represented mip, and represented layer/depth count.
- MSAA unsupported status is now visible without relying on execute_graph_to_resources failure.
- D2Array flattened compatibility and non-base mip represented imports are covered.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 17 passed, 0 failed.

Remaining gap:
- This is capability observability. It does not implement native MSAA texture/sample-level graph execution, simultaneous multi-mip graph IO, native mip/layer/depth RHI addressing, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public graph imported layer/depth subregion upload

Area: RenderGraph public texture import/export layer/depth subregion support.

Status: partial renderer-layer coverage improved.

Implemented:
- Explicit public graph texture import now accepts non-zero array-layer/depth offsets for flattened-compatible represented textures.
- Upload maps layer/depth offset into flattened RHI y offset while preserving x/y subregion offset handling.
- Export writeback still represents the full mip/layer range and reports complete represented coverage when the graph writes the full payload.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 18 passed, 0 failed.

Remaining gap:
- This is flattened layer/depth offset support, not native layer/depth RHI addressing. Simultaneous multi-mip graph IO, partial represented layer/depth export, MSAA imports, backend-wgpu standard-frame/surface graph exports, and broader backend-native resource export integration remain incomplete.

## 2026-05-19 - Public graph buffer import support query

Area: RenderGraph public buffer import capability observability.

Status: partial renderer-layer coverage improved.

Implemented:
- Added Renderer::graph_buffer_import_support and RendererGraphBufferImportSupport.
- The query reports supported status, size, usage, represented byte range, and full-byte-coverage status.
- Subrange-updated public buffers report full-buffer represented import semantics before graph execution.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 18 passed, 0 failed.

Remaining gap:
- This is capability observability. It does not implement minimal dirty-range buffer upload scheduling, partial buffer export ranges, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public graph aggregate import support query

Area: RenderGraph public import capability observability.

Status: partial renderer-layer coverage improved.

Implemented:
- Added Renderer::graph_import_support and RendererGraphImportSupport.
- Aggregate helpers report unsupported texture imports, unsupported buffer imports, total unsupported imports, and all-imports-supported status.
- Mixed graph preflight now reports supported flattened textures, supported buffers, and unsupported MSAA textures without executing graph work.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 18 passed, 0 failed.

Remaining gap:
- This is aggregate capability observability. It does not implement native MSAA texture/sample-level graph execution, simultaneous multi-mip graph IO, native mip/layer/depth RHI addressing, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public graph import preflight execution gate

Area: RenderGraph public import validation / error reporting.

Status: partial renderer-layer coverage improved.

Implemented:
- execute_graph_to_resources now validates aggregate graph import support before building RHI imports.
- Unsupported imports fail with a public preflight error that includes unsupported count and support reason.
- MSAA imported texture rejection now uses the same public support reason exposed by graph_texture_import_support and graph_import_support.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 18 passed, 0 failed.

Remaining gap:
- This improves validation/error consistency. It does not implement native MSAA texture/sample-level graph execution, simultaneous multi-mip graph IO, native mip/layer/depth RHI addressing, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public graph texture import represented layout support metadata

Area: RenderGraph public texture import capability observability.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphTextureImportSupport now includes represented width, height, byte length, and complete mip/layer/subresource coverage flags.
- Support query now uses the same represented-layout calculation as execute_graph_to_resources.
- MSAA, flattened D2Array, and non-base mip represented imports are covered.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 18 passed, 0 failed.

Remaining gap:
- This improves preflight accuracy and tooling metadata. It does not implement native MSAA texture/sample-level graph execution, simultaneous multi-mip graph IO, native mip/layer/depth RHI addressing, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Generated mip-chain import support query coverage

Area: RenderGraph public texture import capability observability.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphTextureImportSupport now has focused coverage for generated mip-chain textures.
- The support query reports the descriptor's full mip count while making the represented import layout explicit as base mip only.
- Generated mip-chain imports report represented_mip = 0, base extent, base-mip byte length, complete layer coverage, incomplete mip coverage, and incomplete subresource coverage.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 18 passed, 0 failed.

Remaining gap:
- This is preflight/observability coverage for the current represented-base-mip import path. It does not implement simultaneous multi-mip graph IO, native mip/layer/depth RHI addressing, native MSAA texture/sample-level graph execution, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Generated mip-chain graph writeback regeneration

Area: RenderGraph public texture import/export generated mip support.

Status: partial renderer-layer coverage improved.

Implemented:
- Imported public textures with generated mip chains now preserve generated-mip semantics after explicit graph writeback.
- The graph still imports the represented base mip for execution, but after a graph pass writes the base mip, the renderer regenerates the full retained mip chain from the updated base bytes.
- RendererGraphTextureExport now reports the regenerated mip-chain subresource byte layout for generated public texture writeback, including complete mip/layer/subresource coverage for the D2 generated chain path.
- Generated mip imports are no longer restricted to the old single-layer D2 branch when uploading the represented base bytes; they use the same flattened-compatible base upload path as other supported texture dimensions.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 18 passed, 0 failed.

Remaining gap:
- This is generated-mip writeback regeneration around a represented base-mip graph execution. It does not implement simultaneous multi-mip graph pass IO, native backend mip/layer/depth addressing, native MSAA texture/sample-level graph execution, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Layered generated mip-chain graph writeback coverage

Area: RenderGraph public texture import/export generated mip support.

Status: partial renderer-layer coverage improved.

Implemented:
- Added focused coverage for generated TextureDimension::D2Array mip-chain imports through explicit public graph execution.
- The graph imports and writes the flattened base-layer representation, then renderer writeback regenerates the retained mip chain per layer.
- RendererGraphTextureExport reports three packed mip subresources for the layered generated chain, with complete mip/layer/subresource coverage and correct byte offsets.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 19 passed, 0 failed.

Remaining gap:
- This validates flattened layered generated-mip writeback for D2Array. It does not implement simultaneous multi-mip graph pass IO, native backend mip/layer/depth addressing, native MSAA texture/sample-level graph execution, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Volume generated mip-chain graph writeback coverage

Area: RenderGraph public texture import/export generated mip support.

Status: partial renderer-layer coverage improved.

Implemented:
- Added focused coverage for generated TextureDimension::D3 mip-chain imports through explicit public graph execution.
- The graph imports and writes the flattened base-volume representation, then renderer writeback regenerates the retained volume mip chain from the updated base volume.
- RendererGraphTextureExport coverage calculation now evaluates expected layer/depth coverage per mip through descriptor-aware mip depth/layer counts, so complete D3 mip chains with shrinking depth are no longer misreported as incomplete.
- The D3 export reports three packed mip subresources with depth counts 4, 2, and 1 plus complete mip/layer/subresource coverage.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 20 passed, 0 failed.

Remaining gap:
- This validates flattened volume generated-mip writeback and descriptor-aware coverage metadata. It does not implement simultaneous multi-mip graph pass IO, native backend mip/layer/depth addressing, native MSAA texture/sample-level graph execution, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Cube generated mip-chain graph writeback coverage

Area: RenderGraph public texture import/export generated mip support.

Status: partial renderer-layer coverage improved.

Implemented:
- Added focused coverage for generated TextureDimension::Cube mip-chain imports through explicit public graph execution.
- The graph imports and writes the flattened base-face representation, then renderer writeback regenerates the retained cube mip chain per face.
- RendererGraphTextureExport reports packed cube mip subresources with complete mip/layer/subresource coverage for all six faces.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 21 passed, 0 failed.

Remaining gap:
- This validates flattened cube generated-mip writeback and export metadata. It does not implement simultaneous multi-mip graph pass IO, native backend cube face/mip addressing, native MSAA texture/sample-level graph execution, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - CubeArray generated mip-chain graph writeback coverage

Area: RenderGraph public texture import/export generated mip support.

Status: partial renderer-layer coverage improved.

Implemented:
- Added focused coverage for generated TextureDimension::CubeArray mip-chain imports through explicit public graph execution.
- The graph imports and writes the flattened base-face/layer representation, then renderer writeback regenerates the retained cube-array mip chain per face/layer.
- RendererGraphTextureExport reports packed cube-array mip subresources with complete mip/layer/subresource coverage for twelve faces/layers.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 22 passed, 0 failed.

Remaining gap:
- This validates flattened cube-array generated-mip writeback and export metadata. It does not implement simultaneous multi-mip graph pass IO, native backend cube-array face/layer/mip addressing, native MSAA texture/sample-level graph execution, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - D1 generated mip-chain graph writeback coverage

Area: RenderGraph public texture import/export generated mip support.

Status: partial renderer-layer coverage improved.

Implemented:
- Added focused coverage for generated TextureDimension::D1 mip-chain imports through explicit public graph execution.
- The graph imports and writes the base line representation, then renderer writeback regenerates the retained D1 mip chain.
- RendererGraphTextureExport reports packed D1 mip subresources with complete mip/layer/subresource coverage.
- Generated mip explicit graph compatibility now has targeted shape coverage for D1, D2, D2Array, D3, Cube, and CubeArray.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 23 passed, 0 failed.

Remaining gap:
- This validates D1 generated-mip writeback and completes current flattened-compatible generated-mip shape coverage. It does not implement simultaneous multi-mip graph pass IO, native backend mip/layer/depth/face addressing, native MSAA texture/sample-level graph execution, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Generated mip-chain packed graph import read support

Area: RenderGraph public texture import/export generated mip support.

Status: partial renderer-layer coverage improved.

Implemented:
- Generated public texture imports now allocate an RHI-compatible packed full-chain texture for explicit graph execution instead of uploading only the base mip.
- The packed representation stacks each mip vertically at x=0 using each mip's flattened layer/depth height, while keeping compact public byte payload ordering and RendererGraphTextureExport subresource offsets aligned.
- Graph passes can read lower generated mips through the packed y offsets exposed by the export/import subresource layout model.
- graph_texture_import_support reports complete mip/layer/subresource coverage for generated full-chain imports and reports the packed represented RHI height for generated textures while preserving logical represented height for non-generated flattened imports.
- Writeback still treats the base mip as authoritative and regenerates the retained mip chain after graph execution, preserving TextureInfo::mips_generated semantics.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 23 passed, 0 failed.

Remaining gap:
- This adds packed full-chain graph import/read support for generated mip chains. It does not implement native multi-mip graph resource addressing, retaining graph writes to lower generated mips, native MSAA texture/sample-level graph execution, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Generated lower-mip graph writeback retention

Area: RenderGraph public texture import/export generated mip support.

Status: partial renderer-layer coverage improved.

Implemented:
- Explicit public graph writeback now reads generated packed mip-chain exports per represented subresource instead of reading a base-width padded rectangle.
- If graph execution changes only the base mip and lower mips remain unchanged, writeback regenerates the retained chain from the new base and keeps TextureInfo::mips_generated true.
- If graph execution authors lower mip bytes, writeback preserves the full packed mip-chain public payload and marks TextureInfo::mips_generated false so public observability does not falsely claim the chain is still generated from the base mip.
- Authored packed mip-chain payloads remain importable through the same packed full-chain graph import path and preserve complete subresource coverage metadata.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 24 passed, 0 failed.

Remaining gap:
- This preserves graph-authored lower mip bytes in the packed compatibility representation. It is still not native backend mip/layer/depth addressing, does not provide named per-mip graph resources, does not implement native MSAA texture/sample-level graph execution, and does not close backend-wgpu standard-frame/surface graph exports.

## 2026-05-19 - Graph texture import support subresource layout metadata

Area: RenderGraph public texture import capability observability.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphTextureImportSupport now exposes subresources with the same byte-layout fields used by RendererGraphTextureExport.
- Generated packed mip-chain imports report every represented mip subresource, including mip level, byte offset, byte length, extent, row pitch, and layer/depth count before graph execution.
- Public preflight tools no longer need to infer import-time packed layout from aggregate represented byte length or descriptor metadata.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 24 passed, 0 failed.

Remaining gap:
- This is public import-layout observability for the packed compatibility path. It does not implement native backend subresource handles, named per-mip graph resources, native MSAA texture/sample-level graph execution, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Graph import support subresource aggregate helpers

Area: RenderGraph public texture import capability observability / tooling helpers.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphTextureImportSupport now exposes subresource_byte_len and has_packed_subresources helpers.
- RendererGraphImportSupport now exposes aggregate texture import helpers for packed-subresource counts, incomplete subresource coverage counts, total represented subresource bytes, all-texture-complete status, and any incomplete import coverage.
- Generated packed mip-chain graph imports verify both per-import helper output and aggregate graph preflight helper output before execution.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 24 passed, 0 failed.

Remaining gap:
- These helpers summarize public import-layout metadata for the packed compatibility path. They do not implement native backend subresource handles, named per-mip graph resources, native MSAA texture/sample-level graph execution, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public texture info subresource layout metadata

Area: Texture/resource observability.

Status: partial renderer-layer coverage improved.

Implemented:
- TextureInfo now exposes retained texture subresources, including mip level, layer/depth range, extent, byte offset, byte length, row pitch, and rows per image.
- TextureInfo reports complete_mip_coverage, complete_layer_coverage, and complete_subresource_coverage for the retained public texture payload.
- TextureInfo exposes subresource_byte_len and has_packed_subresources helpers.
- Authored packed mip-chain writeback coverage verifies TextureInfo can distinguish packed retained public texture bytes from a generated-from-base mip chain.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 24 passed, 0 failed.

Remaining gap:
- This is public resource observability for retained CPU-side/public texture payload layout. It does not implement native backend subresource handles, named per-mip graph resources, native MSAA texture/sample-level graph execution, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public buffer info byte coverage metadata

Area: Buffer/resource observability.

Status: partial renderer-layer coverage improved.

Implemented:
- BufferInfo now exposes retained public buffer byte_offset, byte_len, and complete_byte_coverage.
- BufferInfo exposes represented_byte_len and has_complete_byte_coverage helpers.
- Explicit public graph buffer subrange update/writeback coverage verifies BufferInfo reports full retained byte coverage after graph export writeback.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 24 passed, 0 failed.

Remaining gap:
- This is retained public buffer payload observability. It does not implement minimal dirty-range upload scheduling, partial graph buffer export ranges, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public buffer represented byte-range import support

Area: Buffer/resource import observability and upload behavior.

Status: partial renderer-layer coverage improved.

Implemented:
- Stored public buffers now track a represented byte range instead of treating every update as full-buffer represented data.
- Buffer updates merge into the represented byte range, and explicit graph import uploads only that represented range into a newly created zero-initialized RHI buffer.
- RendererGraphBufferImportSupport and BufferInfo now report represented byte_offset, byte_len, and complete_byte_coverage from the tracked range.
- Graph writeback still reads the full exported buffer and restores full-byte coverage for the retained public buffer.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 24 passed, 0 failed.

Remaining gap:
- This implements a single merged represented upload range for public buffer imports. It does not implement multiple disjoint dirty ranges, backend-resident dirty-range synchronization across persistent native resources, partial graph buffer export ranges, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Public buffer disjoint represented byte-range import support

Area: Buffer/resource import observability and upload behavior.

Status: partial renderer-layer coverage improved.

Implemented:
- Public buffers now retain multiple disjoint represented byte ranges instead of collapsing every update into one uploaded span.
- BufferInfo and RendererGraphBufferImportSupport expose byte_ranges while preserving byte_offset/byte_len as the bounding span.
- BufferInfo::represented_byte_len now sums exact represented ranges rather than the bounding span.
- Explicit graph import uploads each represented range separately, leaving gaps zero-initialized in the graph RHI buffer.
- Graph writeback still restores full-byte coverage after reading back the full exported buffer.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 24 passed, 0 failed.

Remaining gap:
- This covers retained public buffer disjoint range import into newly created explicit graph RHI buffers. It does not implement persistent backend dirty-range synchronization across frames/resources, partial graph buffer export ranges, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Graph buffer import represented-range aggregate helpers

Area: RenderGraph public buffer import capability observability / tooling helpers.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphBufferImportSupport now exposes represented_byte_len and has_disjoint_byte_ranges helpers.
- RendererGraphImportSupport now exposes aggregate buffer import helpers for incomplete byte coverage counts, disjoint-range counts, represented byte totals, and all-buffer-complete status.
- Disjoint public buffer range imports verify both per-buffer helper output and aggregate graph preflight helper output before execution.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 24 passed, 0 failed.

Remaining gap:
- These helpers summarize retained public buffer import metadata for explicit graph imports. They do not implement persistent backend dirty-range synchronization, partial graph buffer export ranges, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Graph buffer export byte-range metadata alignment

Area: RenderGraph public buffer export observability.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphBufferExport now exposes byte_ranges in addition to byte_offset/byte_len and complete_byte_coverage.
- RendererGraphBufferExport exposes represented_byte_len and has_disjoint_byte_ranges helpers.
- Promoted transient buffer exports and imported public buffer writebacks report a single full-buffer byte range today, matching retained BufferInfo after writeback.
- Existing full-buffer writeback semantics remain explicit while preserving a stable payload shape for future partial buffer exports.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 24 passed, 0 failed.

Remaining gap:
- This aligns export metadata with import/resource range observability. It does not implement partial graph buffer export ranges, persistent backend dirty-range synchronization, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Partial graph buffer export ranges

Area: RenderGraph public buffer export/readback semantics.

Status: partial renderer-layer coverage improved.

Implemented:
- RenderGraphBuilder now exposes export_buffer_range(label, buffer, byte_offset, byte_len) alongside full export_buffer.
- RhiBufferExport carries byte_offset and byte_len so facade promotion/writeback can read only the requested export range.
- Imported public buffer range exports update only the exported public byte range and mark the retained BufferInfo range as incomplete coverage.
- Promoted transient buffer range exports create a public BufferHandle of the graph buffer size, copy only the exported range into retained bytes, and expose incomplete byte coverage through RendererGraphBufferExport and BufferInfo.
- RendererGraphBufferExport byte_ranges/helper metadata reports the partial range consistently with retained BufferInfo.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 25 passed, 0 failed.

Remaining gap:
- This closes explicit graph single-range buffer exports. It does not implement multiple export ranges per graph buffer, persistent backend dirty-range synchronization, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Imported buffer partial export range preflight

Area: RenderGraph public buffer export validation / error reporting.

Status: partial renderer-layer coverage improved.

Implemented:
- RenderGraphBuilder exposes exported_buffer_entries so the renderer facade can preflight public buffer export ranges before graph execution.
- execute_graph_to_resources now validates imported public buffer export ranges against the retained public buffer size before creating graph imports or executing passes.
- Out-of-bounds imported public buffer export ranges fail with RenderGraphValidation before pass execution, including the export label in the user-visible error.
- Graph validation also rejects empty explicit buffer export ranges and continues to validate transient graph buffer export ranges against graph buffer descriptors.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 26 passed, 0 failed.

Remaining gap:
- This closes single-range imported public buffer export preflight. It does not implement multiple export ranges per buffer, persistent backend dirty-range synchronization, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Multiple graph buffer export ranges

Area: RenderGraph public buffer export/readback semantics.

Status: partial renderer-layer coverage improved.

Implemented:
- RenderGraphBuilder now exposes export_buffer_ranges(label, buffer, ranges) for multiple disjoint byte ranges on one exported graph buffer.
- RhiBufferExport carries byte_ranges so facade promotion/writeback can read back each requested range separately.
- Imported public buffer multi-range exports update only those public byte ranges and report a bounding byte_offset/byte_len plus exact byte_ranges.
- Promoted transient multi-range exports create a public BufferHandle of the graph buffer size, copy only requested ranges, and expose disjoint byte_ranges through RendererGraphBufferExport and BufferInfo.
- Single-range export_buffer_range and full export_buffer remain supported.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 26 passed, 0 failed.

Remaining gap:
- This closes explicit graph multi-range buffer exports in the packed/retained public resource model. It does not implement persistent backend dirty-range synchronization, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Buffer export range validation errors

Area: RenderGraph public buffer export validation / error reporting.

Status: partial renderer-layer coverage improved.

Implemented:
- export_buffer_ranges with an empty range list now fails validation instead of silently becoming a full-buffer export.
- Transient graph buffer multi-range exports validate every requested range against the graph buffer descriptor.
- Imported public buffer multi-range exports continue to validate every requested range against the retained public buffer size before graph execution.
- User-visible RenderGraphValidation errors include the export label for empty and out-of-bounds range failures.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 27 passed, 0 failed.

Remaining gap:
- This closes explicit graph buffer export range validation. It does not implement persistent backend dirty-range synchronization, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Graph buffer export range aggregate helpers

Area: RenderGraph public buffer export observability / tooling helpers.

Status: partial renderer-layer coverage improved.

Implemented:
- RendererGraphResourceExports now exposes buffer_exports_with_disjoint_byte_ranges.
- RendererGraphResourceExports now exposes buffer_export_represented_bytes for the sum of exact exported buffer byte ranges.
- Imported public multi-range buffer exports and promoted transient multi-range buffer exports verify aggregate helper values.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 27 passed, 0 failed.

Remaining gap:
- These helpers summarize explicit graph buffer export metadata. They do not implement persistent backend dirty-range synchronization, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.

## 2026-05-19 - Buffer export range normalization

Area: RenderGraph public buffer export/readback semantics.

Status: partial renderer-layer coverage improved.

Implemented:
- export_buffer_ranges now canonicalizes requested byte ranges by sorting them and merging overlapping or adjacent ranges.
- RendererGraphBufferExport, BufferInfo, and aggregate represented-byte helpers report normalized ranges instead of double-counting repeated or adjacent requested ranges.
- Imported public buffer multi-range export coverage now verifies unordered adjacent input ranges normalize to the expected retained/exported byte_ranges.

Validation:
- C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture passed: 27 passed, 0 failed.

Remaining gap:
- This normalizes explicit graph buffer export ranges in the retained/facade model. It does not implement persistent backend dirty-range synchronization, backend-wgpu standard-frame/surface graph exports, or broader backend-native resource export integration.
## 2026-05-19 update: RenderGraph D1/D2/layered texture region exports

Status: `Implemented` for public RenderGraph D1/D2 base-mip texture region exports, D2 non-base mip region exports, D2Array single-layer and multi-layer non-base mip region exports, Cube/CubeArray single-face and aligned multi-layer non-base mip region exports, D3 non-base mip depth-slice region exports, D1/D2 generated base/lower-mip region exports, aligned whole-layer D2Array/D3/Cube/CubeArray base-mip region exports, and single-layer/cross-layer partial-layer flattened D2Array/D3/Cube/CubeArray region exports on the headless/RHI graph-to-resource path.

Evidence:

- Public graph API exposes `RenderGraphBuilder::export_texture_region(label, texture, x, y, width, height)`.
- Public renderer API exposes `Renderer::graph_texture_region_export_support(texture, region)` so tooling can preflight texture region export support and inspect the exact subresource metadata that would be reported before executing a graph.
- Public renderer API exposes `Renderer::graph_region_export_support(&graph)` so tooling can batch-preflight imported public texture region exports declared by a graph, preserve export labels, and aggregate supported/unsupported counts plus subresource byte totals before execution.
- `Renderer::graph_import_support(&graph)` now includes imported public texture region export preflight results alongside texture/buffer import support, so one graph-level support query can surface both import coverage and imported texture region export gates.
- RHI export metadata carries `RhiTextureExportRegion` through compiled graph exports and `RhiTextureExport`.
- Public `RendererGraphTextureExport` now exposes `region: Option<RhiTextureExportRegion>`, so callers can directly observe the requested export rectangle instead of inferring it only from subresource metadata.
- Public `RendererGraphTextureExport` now exposes `subresource_byte_len()` and `has_packed_subresources()` helpers, while `RendererGraphResourceExports` exposes aggregate packed-subresource export counts and exported texture subresource bytes for graph execution results.
- Public `RendererGraphResourceExports` exposes `texture_exports()`, `buffer_exports()`, `promoted_texture_exports()`, `promoted_buffer_exports()`, `promoted_texture_export_labels()`, `promoted_buffer_export_labels()`, `imported_texture_exports()`, `imported_buffer_exports()`, `imported_texture_export_labels()`, `imported_buffer_export_labels()`, `export_count()`, `promoted_export_count()`, `imported_export_count()`, `export_label_count()`, `promoted_export_label_count()`, `imported_export_label_count()`, `texture_region_export_label_count()`, `has_complete_texture_region_export_label_coverage()`, `has_complete_export_label_coverage()`, `texture_exports_with_regions()`, and `has_texture_region_exports()` aggregate helpers for graph-to-resource export observability.
- `RenderGraphStats` now reports `exported_texture_regions` and `exported_texture_region_labels`, exposes `has_texture_region_exports()`, label-count/label-coverage helpers, sorted label helpers, and facade/backend graph-stat merging preserves those fields for frame/debug/capture observability.
- `RenderGraphStats` now reports `backend_exported_texture_regions` and `backend_exported_texture_region_labels`, exposes backend-specific label count/coverage helpers and sorted label helpers, and facade/backend graph-stat merging preserves backend-origin texture region export provenance instead of only folding native backend region exports into aggregate counts.
- `FrameStats`, `FrameProfile`, `FrameDebugReport`, and `FrameCapture` now directly report public graph exported texture/buffer counts and labels, so tooling can observe explicit graph export labels without parsing nested `public_graph_execution`.
- `FrameStats`, `FrameProfile`, `FrameDebugReport`, and `FrameCapture` now directly report promoted public graph texture/buffer export counts and labels, so tooling can distinguish transient-promoted graph outputs from imported-resource exports without parsing nested `public_graph_execution`.
- `FrameStats`, `FrameProfile`, `FrameDebugReport`, and `FrameCapture` now directly report imported public graph texture/buffer export counts and labels, so tooling can distinguish imported-resource exports from promoted graph outputs without parsing nested `public_graph_execution`.
- `FrameCapture` mirrors the `FrameStats` public graph export flat fields directly, preventing capture payloads from drifting by recalculating region-export observability from nested graph execution data.
- `FrameStats::{public_graph_export_count, public_graph_promoted_export_count, public_graph_imported_export_count, public_graph_export_label_count, public_graph_promoted_export_label_count, public_graph_imported_export_label_count, public_graph_texture_region_export_label_count, has_complete_public_graph_texture_region_export_label_coverage, has_complete_public_graph_export_label_coverage}` provide immediate-frame aggregate helper coverage for public graph export tooling.
- `FrameProfile`, `FrameDebugReport`, and `FrameCapture` expose matching public graph export aggregate helpers, keeping profile/debug/capture tooling aligned with `FrameStats`.
- `FrameStats` now directly reports `public_graph_texture_region_exports` and `public_graph_texture_region_export_labels`, so immediate frame output can observe explicit public graph texture region exports without parsing nested graph execution payloads.
- `FrameProfile` now directly reports `public_graph_texture_region_exports` and `public_graph_texture_region_export_labels`, so profiling payloads preserve the same explicit public graph texture region export observability as stats/debug/capture.
- `FrameCaptureResourceDump` now reports `public_graph_exported_texture_labels`, `public_graph_exported_buffer_labels`, `public_graph_promoted_texture_labels`, `public_graph_promoted_buffer_labels`, `public_graph_imported_texture_export_labels`, `public_graph_imported_buffer_export_labels`, `public_graph_texture_region_exports`, and `public_graph_texture_region_export_labels`, so internal captures/resource dumps expose public graph export counts, labels, and promoted/imported classification from the last public graph execution.
- `FrameCaptureResourceDump::{public_graph_export_count, public_graph_promoted_export_count, public_graph_imported_export_count, public_graph_export_label_count, public_graph_promoted_export_label_count, public_graph_imported_export_label_count, public_graph_texture_region_export_label_count, has_complete_public_graph_texture_region_export_label_coverage, has_complete_public_graph_export_label_coverage}` provide aggregate helper coverage for capture tooling.
- `FrameDebugReport` now directly reports `public_graph_texture_region_exports` and `public_graph_texture_region_export_labels`, so editor/debug tooling can observe explicit public graph texture region exports without parsing nested graph execution payloads.
- `FrameCapture` now directly reports `public_graph_texture_region_exports` and `public_graph_texture_region_export_labels`, keeping capture payloads aligned with frame debug reports for explicit public graph texture region exports.
- Promoted transient texture exports read back only the requested D2 rectangle, create a public texture with the original graph extent, and report partial `subresources` plus incomplete subresource coverage.
- Imported public D2 texture exports write back only the requested rectangle into the public resource payload and report partial `TextureInfo` coverage.
- Imported public D2 non-base mip exports accept region coordinates in represented graph/RHI mip space and map them back to public `mip_level`, `offset`, and extent metadata.
- Imported public D2Array single-layer and aligned multi-layer non-base mip exports accept aligned layer-height regions and map them back to public `mip_level`, `base_layer`, `layer_count`, `offset`, and extent metadata.
- Imported public Cube/CubeArray single-face and aligned multi-layer non-base mip exports accept aligned face-height regions and map them back to public `mip_level`, `base_layer`, `layer_count`, `offset`, and extent metadata.
- Imported public D3 non-base mip exports accept flattened depth-slice regions aligned to the mip slice height and map them back to public `mip_level`, `base_layer`, `layer_count`, `offset`, and extent metadata.
- Imported public D1/D2 generated mip-chain exports accept regions inside a single generated packed mip, write back only that mip-local region, map packed RHI y coordinates back to public `mip_level` and `offset` metadata, and clear `mips_generated` because full-chain completeness is no longer represented.
- Imported public D1 texture exports support `y=0,height=1` rectangular range writeback and report partial `TextureInfo` coverage when width is incomplete.
- Imported public D2Array/D3/Cube/CubeArray texture exports support regions whose flattened `y` and `height` are aligned to full layer/depth-slice height; metadata maps them back to `base_layer`, `layer_count`, public `offset`, and extent.
- Imported public D2Array/D3/Cube/CubeArray texture exports also support partial flattened regions across one or more layers/depth-slices; metadata maps flattened `y` back to mip-local `offset.y` and splits cross-layer partial exports into multiple public subresources when one RHI region cannot be represented by a single public subresource.
- Cross-layer partial flattened exports retain multi-subresource layout metadata on the public texture so the texture can be imported into a later public graph and uploaded back to the correct flattened RHI coordinates.
- `RendererGraphTextureRegionExportSupport` reports supported/unsupported status, unsupported reason, region, texture descriptor fields, flattened-RHI compatibility, export subresources, subresource byte length, packed-subresource status, and mip/layer/subresource coverage flags for public texture region export preflight.
- `RendererGraphRegionExportSupport` aggregates graph-level imported texture region export preflight results with `texture_region_exports()`, `supported_texture_region_exports()`, `unsupported_texture_region_exports()`, `has_texture_region_exports()`, `has_unsupported_texture_region_exports()`, `all_texture_region_exports_supported()`, `texture_region_export_subresource_bytes()`, sorted label-list helpers, unsupported-reason aggregation, reason count/bool helpers, label+reason summary helpers, and complete label-coverage checks.
- `RendererGraphImportSupport` exposes matching imported texture region export aggregate helpers, boolean gate helpers, label/reason lists, reason count/bool helpers, and label+reason summary helpers, keeping the existing graph import-support entry point aligned with explicit region export gates.
- Imported public texture region export preflight rejects invalid D1 y/height, non-base-mip, multisampled, overflowed, and out-of-bounds regions before pass execution.
- Coverage calculation now treats mip/layer coverage and 2D extent coverage separately, so a base-mip partial rectangle is no longer reported as complete subresource coverage.
- `RendererGraphTextureExportSubresource` and `TextureInfoSubresource` now expose `offset`, so partial region exports are observable as position plus extent instead of extent-only metadata.
- `RendererGraphTextureImportSupport.subresources` reuses the same offset-aware metadata, so graph import support reports the public texture region being imported while separate `represented_*` fields keep the RHI-compatible upload extent visible.

Validation:

```powershell
& 'C:\Users\JM\.cargo\bin\cargo.exe' test -p engine_renderer execute_graph_to_resources_ -- --nocapture
```

Result: `49 passed; 0 failed; 0 ignored; 273 filtered out`.

New focused tests:

- `execute_graph_to_resources_promotes_partial_texture_export_regions`
- `execute_graph_to_resources_writes_back_imported_texture_export_regions`
- `execute_graph_to_resources_rejects_imported_texture_export_regions_out_of_bounds`
- `execute_graph_to_resources_writes_back_imported_d1_texture_export_regions`
- `execute_graph_to_resources_rejects_imported_d1_texture_export_regions_with_y_extent`
- `execute_graph_to_resources_writes_back_imported_layered_texture_export_regions`
- `execute_graph_to_resources_writes_back_imported_layered_partial_layer_texture_export_regions`
- `execute_graph_to_resources_writes_back_imported_layered_cross_layer_partial_texture_export_regions`
- `execute_graph_to_resources_rejects_imported_layered_texture_export_regions_out_of_bounds`
- `execute_graph_to_resources_writes_back_imported_non_base_mip_texture_export_regions`
- `execute_graph_to_resources_rejects_imported_non_base_mip_texture_export_regions_out_of_bounds`
- `execute_graph_to_resources_writes_back_generated_mip_base_texture_export_regions`
- `execute_graph_to_resources_writes_back_generated_lower_mip_texture_export_regions`
- `execute_graph_to_resources_rejects_generated_mip_texture_export_regions_outside_chain`
- `execute_graph_to_resources_writes_back_imported_d3_non_base_mip_texture_export_regions`
- `execute_graph_to_resources_rejects_imported_d3_non_base_mip_texture_export_regions_out_of_bounds`
- `execute_graph_to_resources_writes_back_generated_d1_mip_base_texture_export_regions`
- `execute_graph_to_resources_rejects_generated_d1_mip_texture_export_regions_with_y_extent`
- `execute_graph_to_resources_writes_back_imported_d2_array_non_base_mip_texture_export_regions`
- `execute_graph_to_resources_writes_back_imported_d2_array_non_base_mip_multi_layer_texture_export_regions`
- `execute_graph_to_resources_writes_back_imported_cube_non_base_mip_texture_export_regions`
- `execute_graph_to_resources_writes_back_imported_cube_non_base_mip_multi_layer_texture_export_regions`
- `graph_texture_region_export_support_reports_partial_layer_boundaries`
- `graph_region_export_support_reports_imported_texture_region_export_batch`
- Region export tests assert public `offset` metadata for promoted transient, imported D2, and imported D1 partial exports.
- Region export tests assert `RendererGraphTextureExport.region` is `None` for full exports and `Some(...)` for partial texture exports.
- Region export tests assert aggregate region-export counts for full and partial texture exports.
- Region export tests assert graph execution stats carry region-export count and labels.
- `builder_tracks_texture_region_export_stats` asserts graph compile stats and compiled resource exports preserve region metadata.
- `accumulate_graph_stats_preserves_import_export_resource_observability` and `facade_backend_graph_merge_preserves_semantic_and_native_execution_stats` assert region-export stats survive accumulation and facade/backend merge.
- `accumulate_graph_stats_preserves_import_export_resource_observability` asserts texture-region and backend-specific texture-region export counts, labels, sorted labels, and label coverage helpers survive graph-stat accumulation.
- `facade_backend_graph_merge_preserves_semantic_and_native_execution_stats` asserts backend-origin texture region export counts, labels, sorted labels, and label coverage helpers are visible through aggregate and backend-specific graph stats after facade/backend graph-stat merge.
- `frame_capture_resource_dump_counts_public_graph_exports` asserts capture resource dumps count and label public graph texture exports, buffer exports, texture region exports, promoted exports, imported-resource exports, `RendererGraphResourceExports` aggregate helpers, stats/profile/debug/capture aggregate helpers, dump aggregate count helpers, aggregate label-coverage helpers, texture-region-specific label coverage helpers, and non-zero imported texture/buffer export observability through `FrameStats`, `FrameProfile`, `FrameDebugReport`, `FrameCapture`, resource dump labels, and public texture/buffer writeback.
- `frame_debug_report_and_capture_expose_public_graph_export_handles` asserts explicit public graph texture/buffer export flat counts and labels, promoted export flat counts and labels, imported export zero/empty flat fields, `RendererGraphResourceExports` aggregate helpers, stats/profile/debug/capture aggregate helpers, texture region export handles, region readback bytes, `FrameStats::public_graph_execution`, direct `FrameStats` region-export fields, direct `FrameProfile` region-export fields, `FrameCapture::public_graph_execution`, direct `FrameDebugReport` region-export fields, and direct `FrameCapture` region-export fields stay consistent.
- Layered region tests cover D2Array, D3, Cube, and CubeArray aligned whole-layer exports, single-layer partial-layer flattened exports, cross-layer partial flattened exports split into multiple public subresources, execution-result packed-subresource helpers, re-import/upload of multi-subresource public texture bytes, and out-of-bounds rejection.
- `graph_texture_region_export_support_reports_partial_layer_boundaries` asserts the public region-export support query reports supported single-layer partial-layer metadata, supported cross-layer multi-subresource metadata, and unsupported out-of-bounds boundaries before graph execution.
- `graph_region_export_support_reports_imported_texture_region_export_batch` asserts graph-level support preflight and `graph_import_support(&graph)` report deterministic imported texture region export labels, supported/unsupported label lists, unsupported reasons, reason count/bool helpers, label+reason summaries, label coverage, supported/unsupported counts, boolean unsupported gates, aggregate bytes, supported subresource metadata, and unsupported out-of-bounds reasons.
- `execute_graph_to_resources_uploads_non_base_mip_subregions` asserts offset-aware graph texture import support for both direct texture support and graph-level import support.

Remaining scope:

- Region export is intentionally scoped to D1/D2 base-mip, D2 non-base mip, D2Array single-layer and multi-layer non-base mip, Cube/CubeArray single-face and aligned multi-layer non-base mip, D3 non-base mip aligned depth-slice regions, D1/D2 generated base/lower-mip, aligned whole-layer D2Array/D3/Cube/CubeArray base-mip, and single-layer partial-layer flattened public texture exports for this slice.
- Cross-layer arbitrary partial-layer flattened region execution/readback is implemented and verified for headless/RHI and backend-wgpu RGBA8 public texture region exports. Remaining scope is native graph-created multi-mip/layer/depth/MSAA transient texture promotion and broader platform surface integration.

## 2026-05-19 - Backend-wgpu surface readback public frame outputs

Area: Public frame output / backend-wgpu surface output.

Status: Partial.

Current evidence:

- `graphics_wgpu::WgpuSurface` now configures swapchain textures with `COPY_SRC` when the surface reports that usage as supported.
- `WgpuSurface` records a pending surface-frame readback after a rendered frame instead of immediately waiting inside `render_frame()`.
- `WgpuSurface::try_resolve_pending_frame_readback()` polls pending readback without blocking and stores the latest `WgpuFrameReadback` only when the map callback has completed.
- `WgpuSurface::resolve_pending_frame_readback()` remains available as an explicit blocking resolve path for callers that choose to wait.
- Surface readback is no longer enabled implicitly by `WgpuRendererRuntime::with_surface`; callers must opt in through `Renderer::set_surface_frame_readback_enabled(true)` after checking `surface_frame_readback_supported()`.
- `Renderer::surface_frame_readback_supported()`, `Renderer::surface_frame_readback_enabled()`, `Renderer::set_surface_frame_readback_enabled()`, `Renderer::request_surface_frame_readback_next_frame()`, and `Renderer::cancel_surface_frame_readback_next_frame()` expose the public control surface for backend surface readback.
- `Renderer::surface_frame_readback_pending()`, `Renderer::surface_frame_readback_available()`, `Renderer::poll_surface_frame_readback()`, and `Renderer::materialize_surface_frame_readback(label)` expose a public ready/poll/materialize path for completed backend surface readbacks outside the originating frame.
- `render_facade_window_usecase` exposes that public path through `--surface-readback`, prints pending/available/materialized counts through `--print-stats`, and can require a materialized durable surface readback texture through `--require-surface-readback`.
- `request_surface_frame_readback_next_frame()` temporarily enables surface readback for the next successfully finished frame and restores the previous readback state after `Frame::finish()`.
- `FramePublicOutputSource` now distinguishes `BackendMainSurfaceReadback` and `BackendSurfaceReadback`.
- `Renderer::public_frame_output_for_view` materializes backend surface readback bytes into a durable public `TextureHandle` with width, height, format, subresource metadata, and stats/debug/capture-compatible `FramePublicFrameOutput` payload instead of always returning no surface public output.
- `Renderer::public_frame_output_for_view` uses nonblocking try-resolve when it needs to materialize a backend surface public output; if the readback is not ready yet, the output is reported through `unsupported_public_frame_outputs` with `BackendSurfaceReadbackUnavailable` instead of blocking the frame.
- `FrameStats.unsupported_public_frame_outputs`, `FrameDebugReport.unsupported_public_frame_outputs`, and `FrameCapture.unsupported_public_frame_outputs` expose backend surface public-output failures with `BackendSurfaceReadbackUnsupported`, `BackendSurfaceReadbackDisabled`, or `BackendSurfaceReadbackUnavailable` reasons.
- `render_wgpu::MeshRenderer` now keeps the post-process pass and both post-process pipelines depth-compatible when a surface depth attachment exists, so reflected material post-pass submissions no longer hit wgpu render-pass/pipeline depth-format validation when surface readback smoke runs through the sampled post-process path.

Validation:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p graphics_wgpu surface_readback_layout_supports_public_color_formats -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_surface_readback_materializes_durable_public_frame_output -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer unsupported_public_frame_outputs_propagate_to_debug_report_and_capture -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer surface_frame_readback_api_requires_backend_surface_renderer -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe build -p render_facade_window_usecase` passed.
- `.\target\debug\render_facade_window_usecase.exe --smoke-frames 8 --wait-for-gpu --print-stats --surface-readback --require-surface-readback` passed on the local window/surface path with exit 0, printing `public_outputs=1`, `unsupported_public_outputs=0`, and `surface_readback_frame_outputs=1`.

Remaining scope:

- Pending readback is no longer resolved inside every surface render, backend surface public output materialization now uses nonblocking try-resolve, and public ready/poll/materialize APIs exist for completed readbacks outside the originating frame.
- Remaining work is backend-wgpu RenderGraph surface export/promotion, stronger real-device surface integration coverage, and native texture-region graph export/readback.
- If a platform surface does not support `COPY_SRC`, backend surface public output still cannot produce a durable texture, but the reason is now observable through stats/debug/capture.
- This closes surface frame-output readback materialization, not backend-wgpu RenderGraph export/promotion or native texture-region graph export execution.
- Full renderer goal remains open because broader standard 3D completion, backend graph exports, resource lifecycle, examples, and remaining matrix items are still incomplete.

## 2026-05-20 - Backend-wgpu texture-region graph export/readback proof

Area: RenderGraph texture-region export / backend-wgpu RHI execution.

Status: partial renderer-layer coverage improved.

Current evidence:

- `Render/engine_renderer/src/graph.rs` now includes `graph_execute_on_wgpu_exports_texture_region_with_readback`.
- The test creates a transient 4x4 RGBA8 graph texture, writes deterministic bytes through the graph callback using `WgpuRhiDevice`, exports a 2x2 region with `export_texture_region`, executes the graph on backend-wgpu RHI, verifies the `RhiTextureExportRegion` metadata, and reads the exported region back through `WgpuRhiDevice::read_texture_rgba8`.
- `Render/engine_renderer/src/graph.rs` also includes `graph_execute_on_wgpu_exports_float_and_depth_regions_with_readback`.
- The second test verifies backend-wgpu region export/readback for transient RGBA16F, RGBA32F, and Depth32Float graph textures. Depth32Float writes execute through a backend-wgpu depth-only render pass that writes `frag_depth` from a storage buffer, avoiding the forbidden `Queue::write_texture` depth-copy path.
- This adds real backend-wgpu evidence for texture-region export/readback instead of relying only on headless/RHI retained-resource tests.
- `Render/engine_renderer/src/rhi.rs` now implements `WgpuRhiDevice::write_texture_depth32f` with a depth-only render pass and adds `wgpu_rhi_write_texture_depth32f_writes_readable_region` for direct RHI coverage.

Validation:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_wgpu_exports_texture_region_with_readback -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_wgpu_exports_float_and_depth_regions_with_readback -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_rhi_write_texture_depth32f_writes_readable_region -- --nocapture` passed, 1 passed.

Remaining scope:

- This proves backend-wgpu transient graph texture-region export/readback for RGBA8, RGBA16F, RGBA32F, and Depth32Float D2 regions, including real Depth32Float write/readback execution. It does not close backend-wgpu standard-frame/surface graph export/promotion, native multi-mip/layer/depth region addressing, or broader platform surface integration.
- Full renderer goal remains open.

## 2026-05-20 - Backend-wgpu imported public texture writeback proof

Area: RenderGraph public resource import/export / backend-wgpu execution.

Status: partial renderer-layer coverage improved.

Current evidence:

- `Render/engine_renderer/src/lib.rs` now includes `execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_texture_shapes`.
- The test constructs a real `Renderer::new` with `BackendPreference::Wgpu`, so `Renderer::execute_graph_to_resources` uses the backend-wgpu RHI device instead of the headless RHI fallback.
- The test verifies pre-graph upload, graph callback readback, graph write, exported imported-public writeback, export provenance, descriptor metadata, and complete represented subresource coverage for imported public color textures.
- Shape coverage: D1, D2, D2Array, D3, Cube, and CubeArray RGBA8 through the current flattened-compatible base-mip RHI representation.
- Format coverage: D2 RGBA16F, D2 RGBA32F, D2 Depth32Float, and flattened D2Array Depth32Float upload/read/writeback on backend-wgpu.
- `execute_graph_to_resources_wgpu_writes_back_imported_rgba8_texture_export_region`, `execute_graph_to_resources_wgpu_writes_back_imported_float_texture_export_regions`, and `execute_graph_to_resources_wgpu_writes_back_imported_depth_texture_export_region` verify imported public D2 RGBA8, D2 RGBA16F/RGBA32F, and D2 Depth32Float `export_texture_region` writeback on backend-wgpu, including partial public texture bytes/layout and incomplete subresource coverage metadata.
- `execute_graph_to_resources_wgpu_writes_back_imported_layered_rgba8_texture_export_regions` verifies D2Array, D3, Cube, and CubeArray RGBA8 whole-layer/face flattened region writeback on backend-wgpu.
- `execute_graph_to_resources_wgpu_writes_back_imported_cross_layer_rgba8_texture_export_regions` verifies D2Array, D3, Cube, and CubeArray RGBA8 cross-layer/cross-face flattened partial region writeback on backend-wgpu, including multi-subresource public layout metadata.
- `execute_graph_to_resources_wgpu_writes_back_imported_non_base_mip_rgba8_texture_export_region` verifies represented non-base mip D2 RGBA8 region writeback on backend-wgpu, including mip-level metadata and incomplete mip/subresource coverage.
- `execute_graph_to_resources_wgpu_regenerates_generated_mip_imports` verifies a generated D2 RGBA8 mip-chain is uploaded to backend-wgpu as a packed graph import, graph execution sees base/mip1/mip2, mutating only the base mip regenerates lower mips on writeback, and public `mips_generated` remains true.
- `execute_graph_to_resources_wgpu_regenerates_generated_mip_import_shapes` verifies generated D1, D2Array, D3, Cube, and CubeArray RGBA8 mip-chain packed import/writeback/regeneration on backend-wgpu.

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

Remaining scope:

- This closes backend-wgpu imported public texture upload/writeback for the current single-mip and packed/generated-mip flattened-compatible color shapes plus D2 float/depth formats and flattened D2Array depth, including D2 RGBA8/RGBA16F/RGBA32F/Depth32Float partial region writeback, D2Array/D3/Cube/CubeArray RGBA8 whole-layer/face plus cross-layer/cross-face flattened region writeback, represented non-base mip D2 RGBA8 region writeback, and generated D1/D2/D2Array/D3/Cube/CubeArray RGBA8 mip-chain regeneration. It does not close standard-frame/surface graph export/promotion, native simultaneous multi-mip/layer/depth region addressing, native MSAA texture/sample-level graph execution, persistent backend-resident dirty synchronization, or broader platform surface integration.
- Full renderer goal remains open.

## 2026-05-20 - Backend-wgpu transient public graph export promotion proof

Area: RenderGraph public resource export / backend-wgpu execution.

Status: partial renderer-layer coverage improved.

Current evidence:

- `Render/engine_renderer/src/lib.rs` now includes `execute_graph_to_resources_wgpu_promotes_exported_transients_to_public_handles`.
- The test constructs a real `Renderer::new` with `BackendPreference::Wgpu`, creates transient graph texture/buffer resources, writes them through a graph callback on the backend-wgpu RHI path, exports the resources, and verifies `Renderer::execute_graph_to_resources` promotes them into durable public handles.
- The promoted texture path verifies D2 RGBA8, RGBA16F, RGBA32F, and Depth32Float export source/provenance, descriptor metadata, complete represented subresource coverage, and public `texture_bytes`.
- The promoted buffer path verifies export source/provenance, complete byte coverage, and public `buffer_bytes`.
- `execute_graph_to_resources_wgpu_promotes_partial_transient_texture_and_buffer_exports` verifies transient D2 RGBA8/RGBA16F/RGBA32F/Depth32Float texture-region promotion and transient buffer disjoint-range promotion on backend-wgpu, including incomplete coverage metadata and durable public bytes.

Validation:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_promotes_exported_transients_to_public_handles -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_promotes_partial_transient_texture_and_buffer_exports -- --nocapture` passed, 1 passed.

Remaining scope:

- This closes explicit backend-wgpu graph-to-public promotion for transient D2 RGBA8/RGBA16F/RGBA32F/Depth32Float texture and buffer exports, plus partial transient D2 RGBA8/RGBA16F/RGBA32F/Depth32Float texture-region and disjoint buffer-range exports. It does not close backend-wgpu standard-frame/surface graph export/promotion, native multi-shape/multi-mip transient texture promotion, persistent backend-resident graph resources, or broader platform surface integration.
- Full renderer goal remains open.

## 2026-05-20 - Backend-wgpu imported public buffer writeback proof

Area: RenderGraph public buffer import/export / backend-wgpu execution.

Status: partial renderer-layer coverage improved.

Current evidence:

- `Render/engine_renderer/src/lib.rs` now includes `execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_buffer_export`.
- The test constructs a real `Renderer::new` with `BackendPreference::Wgpu`, creates a public buffer with initial bytes, imports it into a graph, verifies the backend-wgpu RHI sees the uploaded bytes before graph mutation, writes updated bytes inside the graph callback, exports the imported graph buffer, and verifies the original public `BufferHandle` contains the updated bytes.
- The execution result verifies `ImportedPublic` provenance, full-buffer byte range metadata, complete byte coverage, and label lookup through `RendererGraphResourceExports`.
- `Render/engine_renderer/src/lib.rs` also includes `execute_graph_to_resources_wgpu_writes_back_imported_buffer_export_ranges`, which verifies partial/disjoint imported buffer export ranges on backend-wgpu.
- `Render/engine_renderer/src/rhi.rs` now stores backend-wgpu RHI buffers with separate logical size and 4-byte-aligned physical size, and handles buffer read/write requests that are not naturally aligned to `wgpu::COPY_BUFFER_ALIGNMENT` by using aligned physical ranges plus caller-visible slicing. This avoids wgpu validation panics for valid renderer-layer byte ranges, including ranges at the end of non-4-byte-sized public buffers.

Validation:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_buffer_export -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_buffer_export_ranges -- --nocapture` passed, 1 passed, including a 7-byte public buffer with end-of-buffer partial export range writeback.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_ -- --nocapture` passed, 4 passed.

Remaining scope:

- This closes explicit backend-wgpu imported public buffer upload/writeback for full-buffer exports and partial/disjoint byte-range exports. It does not close persistent backend-resident dirty-range synchronization, standard-frame/surface graph export/promotion, or broader platform surface integration.
- Full renderer goal remains open.

## 2026-05-20 - Standard-frame graph extension public export promotion

Area: RenderGraph / frame lifecycle / public graph exports.

Status: Partial.

Implemented evidence:
- `build_view_graph_stats` now detects frame/view graph exports and executes the standard view graph through the RHI export path instead of dropping export materialization after stats aggregation.
- Frame graph exports reuse `Renderer::promote_rhi_graph_exports`, so transient texture and buffer exports from public `RenderGraphExtension` instances become durable public `TextureHandle` and `BufferHandle` resources.
- The frame path records the promoted execution into `Renderer::last_graph_execution`, which feeds `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` public graph export/promoted/imported counters and labels.
- When backend-wgpu runtime is active, standard-frame graph export promotion uses the backend-wgpu RHI device; otherwise it uses the headless RHI device.
- A failed frame graph export promotion clears stale `last_graph_execution` before execution, avoiding reuse of older explicit graph export handles.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_exports -- --nocapture` passed, 5 passed.
- The focused filter includes `render_graph_extension_exports_are_visible_in_frame_stats_and_debug_report`, `profiled_render_graph_extension_exports_remain_visible_in_frame_outputs`, `render_graph_extension_exports_promote_to_public_handles_on_wgpu_frame`, `render_graph_extension_exports_aggregate_across_frame_views`, and `render_graph_extension_exports_promote_on_headless_main_surface_target`.

Remaining scope:
- This closes standard-frame graph extension export promotion for headless and backend-wgpu headless frame targets. It does not close native surface/swapchain graph export promotion, native multi-mip/layer/depth region graph addressing, native MSAA texture/sample-level graph execution, persistent backend-resident graph resources, or broader platform surface integration.

Additional aggregation note:
- Frame graph export promotion now aggregates multiple export-producing views within the same renderer frame instead of overwriting `last_graph_execution` with the last view. Explicit `Renderer::execute_graph_to_resources` executions remain separate from frame-graph aggregation.



Frame-index note:
- Same-frame export aggregation uses the active `Frame` index, including `FrameInput::frame_index_override`, rather than the renderer default index.

## 2026-05-20 - Resolved MSAA public graph import compatibility

Area: RenderGraph / public texture import-export / MSAA compatibility.

Status: Partial.

Implemented evidence:
- Public multisampled D2 textures with one mip/layer are now importable by explicit public RenderGraph execution through a resolved single-sample compatibility representation.
- The RHI graph texture remains single-sample because the current RHI texture descriptor has no native sample-count field; the imported bytes represent the resolved public payload that `TextureDesc::initial_data`, `texture_bytes`, `TextureInfo`, and writeback already expose.
- Exporting an imported MSAA public texture writes the resolved graph result back to the same public `TextureHandle` while preserving `TextureInfo.samples` and `RendererGraphTextureExport.samples` as the original multisample count.
- Region exports are supported for the resolved MSAA D2 representation and correctly report partial subresource coverage.
- `graph_texture_import_support` and aggregate `graph_import_support` now report supported MSAA resolved imports instead of treating all multisampled public textures as unsupported.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer msaa -- --nocapture` passed, 6 passed.
- The focused filter includes resolved MSAA import/writeback, backend-wgpu resolved MSAA import/writeback, resolved MSAA texture-region export, and the updated import-support preflight test.

Remaining scope:
- Native graph-created MSAA texture creation, RHI sample-count observability, mode-selectable resolves, and user-supplied custom resolve paths are implemented. Remaining evidence gap is focused compile/test validation where noted.



Additional resolved-MSAA observability:
- `RendererGraphTextureImportSupport`, `RendererGraphTextureRegionExportSupport`, and `RendererGraphTextureExport` now expose `resolved_msaa_compatible` so tooling can distinguish resolved public MSAA compatibility from native sample-level MSAA graph execution.
- `RendererGraphImportSupport::resolved_msaa_texture_imports()` and `RendererGraphResourceExports::resolved_msaa_texture_exports()` expose aggregate counts for resolved-MSAA compatibility paths.
- Focused `msaa` tests now assert the per-resource flag and aggregate helpers for import, export, backend-wgpu import/writeback, and region export.


Additional surface-adjacent promotion evidence:
- `render_graph_extension_exports_promote_on_headless_main_surface_target` verifies that a standard frame targeting `RenderTarget::MainSurface` in the headless/stub path still promotes graph extension texture/buffer exports into durable public handles and reports them through frame public graph stats.
- This covers headless/stub MainSurface target semantics. Readback-backed surface graph export promotion is implemented; direct swapchain image graph export remains a platform/wgpu capability-gated boundary.

Additional frame tooling observability:
- `FrameStats`, `FrameProfile`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` now expose `public_graph_resolved_msaa_texture_exports` and `public_graph_resolved_msaa_texture_export_labels` for the latest public graph execution.
- The resolved MSAA test now verifies these fields through a follow-up frame, internal capture, debug report, and capture resource dump.

Profile validation note:
- The resolved MSAA focused test now also asserts `FrameProfile` resolved-MSAA export count/labels and helper coverage, so every newly exposed frame-tooling surface is covered by the same test filter.

Additional helper parity:
- `FrameDebugReport` and `FrameCapture` now expose `public_graph_resolved_msaa_texture_export_label_count()` matching `FrameStats`, `FrameProfile`, and `FrameCaptureResourceDump`.
- The focused `msaa` filter verifies the debug-report and capture helpers in addition to the raw count/label fields.

Additional graph-region helper parity:
- `RendererGraphRegionExportSupport::resolved_msaa_texture_region_exports()` now mirrors the aggregate helper on `RendererGraphImportSupport`, so direct region-export preflight and aggregate import-support preflight report resolved-MSAA compatibility consistently.
- The focused `msaa` filter verifies both helper surfaces before executing the resolved-MSAA region export graph.

## 2026-05-20 - Window usecase public graph export promotion option

Area: Examples / surface-adjacent frame graph exports / public observability.

Status: Partial.

Implemented evidence:
- `render_facade_window_usecase` now supports `--graph-export` to register a public `RenderGraphExtension` while rendering `RenderTarget::MainSurface` through `Renderer::with_surface`.
- `--require-graph-export` enables the extension and fails the smoke run if no promoted public graph export is reported by the frame stats.
- `--print-stats` now reports public graph export/promoted counts and promoted texture/buffer labels, and the window title includes promoted graph export count.
- The extension exports a transient texture and buffer through the public renderer facade path; this exercises the same frame graph promotion path used by standard frame graph extension exports.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe check -p render_facade_window_usecase` passed.

Remaining scope:
- This is a real window-usecase wiring and build check for MainSurface graph extension exports. It is not native swapchain image export/promotion and does not replace a GUI smoke launch with `--require-graph-export` on a local platform surface.

GUI smoke validation:
- `C:\Users\JM\.cargo\bin\cargo.exe run -p render_facade_window_usecase -- --smoke-frames 8 --wait-for-gpu --print-stats --graph-export --require-graph-export` passed on the local window path with exit 0.
- The smoke output reported `public_graph_exports=2`, `public_graph_promoted_exports=2`, `public_graph_promoted_textures=1`, `public_graph_promoted_buffers=1`, `public_graph_promoted_texture_labels=["facade_window_graph_texture_output"]`, and `public_graph_promoted_buffer_labels=["facade_window_graph_buffer_output"]`.

Combined GUI smoke validation:
- `C:\Users\JM\.cargo\bin\cargo.exe run -p render_facade_window_usecase -- --smoke-frames 8 --wait-for-gpu --print-stats --surface-readback --require-surface-readback --graph-export --require-graph-export` passed on the local window path with exit 0.
- The smoke output reported `public_outputs=1`, `unsupported_public_outputs=0`, `surface_readback_frame_outputs=1`, `public_graph_exports=2`, and `public_graph_promoted_exports=2`, proving surface readback public frame output and graph export promotion coexist in the window usecase.

MainColor graph export validation:
- `render_facade_window_usecase --graph-export` now also exports `RenderGraphExtensionContext::main_color()` as `facade_window_main_color_output`, in addition to its own transient texture/buffer.
- `C:\Users\JM\.cargo\bin\cargo.exe check -p render_facade_window_usecase` passed after this change.
- `C:\Users\JM\.cargo\bin\cargo.exe run -p render_facade_window_usecase -- --smoke-frames 8 --wait-for-gpu --print-stats --surface-readback --require-surface-readback --graph-export --require-graph-export` passed with `public_graph_exports=3`, `public_graph_promoted_exports=3`, `public_graph_promoted_textures=2`, and promoted texture labels `facade_window_main_color_output` plus `facade_window_graph_texture_output`.

Remaining scope:
- This proves standard frame `main_color` graph resource promotion in the window MainSurface usecase. It is still not native swapchain image export/promotion; the swapchain readback path remains the durable surface output mechanism.

MainColor regression test:
- `render_graph_extension_exports_main_color_to_public_handle` verifies that `RenderGraphExtensionContext::main_color()` can be exported by a public graph extension and promoted into a durable public texture handle on the standard frame path.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_exports -- --nocapture` passed, 6 passed.

MainDepth graph export validation:
- `render_graph_extension_exports_main_color_and_depth_to_public_handles` verifies that both `RenderGraphExtensionContext::main_color()` and `main_depth()` can be exported by a public graph extension and promoted into durable public texture handles on the standard frame path. The promoted depth texture reports `TextureFormat::Depth32Float`.
- `render_facade_window_usecase --graph-export` now exports `facade_window_main_depth_output` in addition to main color, an extension texture, and an extension buffer.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_exports -- --nocapture` passed, 6 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe check -p render_facade_window_usecase` passed.
- `C:\Users\JM\.cargo\bin\cargo.exe run -p render_facade_window_usecase -- --smoke-frames 8 --wait-for-gpu --print-stats --surface-readback --require-surface-readback --graph-export --require-graph-export` passed locally with `public_graph_exports=4`, `public_graph_promoted_exports=4`, `public_graph_promoted_textures=3`, and promoted texture labels including `facade_window_main_color_output` and `facade_window_main_depth_output`.

Remaining scope:
- This closes standard-frame context color/depth graph resource promotion in headless/stub and window MainSurface usecase coverage. It still does not expose the native swapchain image itself as a graph resource.

Strict graph-export smoke gate:
- `--require-graph-export` now requires all expected promoted outputs, not merely a non-zero promoted-export count: `facade_window_main_color_output`, `facade_window_main_depth_output`, `facade_window_graph_texture_output`, and `facade_window_graph_buffer_output`.
- `C:\Users\JM\.cargo\bin\cargo.exe check -p render_facade_window_usecase` passed after tightening the gate.
- The combined local smoke run with `--surface-readback --require-surface-readback --graph-export --require-graph-export` passed under the stricter label/count gate.

## 2026-05-20 - Safe graph texture descriptor creation gate

Area: RenderGraph API / native graph texture shape boundaries.

Status: Partial.

Implemented evidence:
- `RenderGraphBuilder::try_create_texture_from_desc(label, TextureDesc)` now validates renderer texture descriptors before creating graph transients.
- The safe API accepts the currently implemented native graph-created texture shape: single-mip, single-sample D1/D2/flattened D2Array/D3/Cube/CubeArray textures.
- Unsupported array/layered, mipped, and multisampled descriptors return `RendererError::RenderGraphValidation` instead of being silently projected into the D2-only `GraphTextureDesc` shape.
- Existing `create_texture_from_desc` is preserved for compatibility, but the new `try_` API gives tools and new code a non-lossy entry point with explicit failure semantics.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed, 1 passed.

Remaining scope:
- This does not implement native graph-created multi-mip/layer/depth/MSAA texture execution. It only exposes a safe descriptor gate and user-visible errors for unsupported native graph texture shapes.

Additional API safety update:
- `RenderGraphBuilder::create_texture_from_desc` is now documented as the legacy descriptor-projection helper and deprecated with guidance to use `try_create_texture_from_desc`.
- The internal builder test path now uses `try_create_texture_from_desc`, keeping focused graph tests aligned with the explicit validation API.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_ -- --nocapture` passed, 5 passed.

## 2026-05-20 coverage matrix update: graph texture descriptor support query

| Area | Status | Evidence | Remaining full-renderer work |
| --- | --- | --- | --- |
| Graph texture descriptor preflight | Implemented | `GraphTextureDescSupport` and `RenderGraphBuilder::texture_desc_support` expose explicit support diagnostics before graph texture creation. | None for current native D2/1-layer/1-mip/1-sample graph shape. |
| Safe graph texture creation | Implemented for current native shape | `try_create_texture_from_desc` validates through the same support query path. | Extend native graph resources to array/depth textures, mip chains, and MSAA before marking those descriptor shapes complete. |
| Legacy descriptor projection | Compatibility only | `create_texture_from_desc` remains available but deprecated. | Remove reliance on width/height/format-only projection once downstream code has migrated. |


## 2026-05-20 coverage correction: cross-layer partial texture region exports

Area: RenderGraph / public imported layered texture region exports.

Status: Implemented for current public imported texture flattened-region model on headless/RHI and backend-wgpu RGBA8.

Evidence:
- `cargo test -p engine_renderer cross_layer -- --nocapture` passed, 2 passed.
- `execute_graph_to_resources_writes_back_imported_layered_cross_layer_partial_texture_export_regions` covers cross-layer partial region writeback, multi-layout public metadata, and reimport/readback of the exported rows.
- `execute_graph_to_resources_wgpu_writes_back_imported_cross_layer_rgba8_texture_export_regions` covers backend-wgpu execution/readback for RGBA8 cross-layer flattened regions.

Remaining full-renderer scope:
- Native graph-created multi-mip/layer/depth/MSAA transient texture promotion remains open.
- Readback-backed surface graph export promotion is implemented; direct swapchain image graph export remains a platform/wgpu capability-gated boundary.
- Persistent backend-resident graph resource synchronization remains open.


## 2026-05-20 - Graph-created D1 transient descriptor and promotion

Area: RenderGraph API / native graph-created texture shape boundaries.

Status: Implemented for D1 and D2 single-layer, single-mip, single-sample graph-created transient textures.

Implemented evidence:
- `GraphTextureRendererDesc` records the renderer-level descriptor shape for textures created through `RenderGraphBuilder::try_create_texture_from_desc`.
- `RenderGraphBuilder::texture_desc_support` now accepts valid D1 descriptors in addition to single-layer D2 descriptors, while continuing to reject array/layered, mipped, and multisampled native graph-created descriptors.
- `Renderer::execute_graph_to_resources` transient texture promotion now respects recorded renderer descriptor metadata, so a graph-created D1 transient exported through the graph is promoted into a durable public `TextureHandle` with `TextureDimension::D1` instead of being forced to D2 metadata.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_graph_created_d1_texture_desc -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed, 1 passed.

Remaining full-renderer scope:
- Native surface/swapchain graph export promotion and persistent backend-resident graph resource synchronization remain open.


## 2026-05-20 - Graph-created D2Array transient descriptor and promotion

Area: RenderGraph API / native graph-created texture shape boundaries.

Status: Implemented for flattened D2Array graph-created transient textures with one mip level and one sample.

Implemented evidence:
- `RenderGraphBuilder::texture_desc_support` now accepts valid D2Array descriptors in addition to D1 and single-layer D2 descriptors.
- `RenderGraphBuilder::try_create_texture_from_desc` records the D2Array renderer descriptor in `GraphTextureRendererDesc` while the RHI execution path materializes it as a flattened 2D texture (`height * layer_count`).
- RHI export metadata now uses the recorded renderer descriptor's flattened RHI height, so full transient D2Array exports read back all layers instead of only the first public layer.
- `Renderer::execute_graph_to_resources` promotes a graph-created D2Array transient into a durable public `TextureHandle` with `TextureDimension::D2Array`, public width/height/layer metadata, complete subresource coverage, and the full flattened byte payload.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_graph_created_d2_array_texture_desc -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed, 1 passed.

Remaining full-renderer scope:
- Native surface/swapchain graph export promotion and persistent backend-resident graph resource synchronization remain open.


## 2026-05-20 - Graph-created D3/Cube/CubeArray transient descriptors and promotion

Area: RenderGraph API / native graph-created texture shape boundaries.

Status: Implemented for one-mip, one-sample graph-created D3, Cube, and CubeArray transient textures through the current flattened RHI representation.

Implemented evidence:
- `GraphTextureRendererDesc::rhi_height` now computes flattened backing height for D2Array, D3, Cube, and CubeArray graph-created transient textures.
- `RenderGraphBuilder::texture_desc_support` and `try_create_texture_from_desc` now accept valid D3, Cube, and CubeArray descriptors while preserving their renderer-level shape metadata.
- RHI graph execution and `RhiTextureExport.desc` use the flattened backing height for these descriptors, so full exports read back all depth slices/faces/layers.
- `Renderer::execute_graph_to_resources` promotes these graph-created transients back into durable public textures with the original public `TextureDimension`, extent, depth/layer count, complete subresource coverage, and byte payload.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_graph_created_d3_texture_desc -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_graph_created_cube_texture_desc -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_graph_created_cube_array_texture_desc -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed, 1 passed.

Remaining full-renderer scope:
- Native surface/swapchain graph export promotion and persistent backend-resident graph resource synchronization remain open.


## 2026-05-20 - Graph-created packed mip-chain transient descriptors and promotion

Area: RenderGraph API / native graph-created texture shape boundaries / mip-chain execution.

Status: Implemented for graph-created one-sample packed mip-chain transients on the headless/RHI path for D1, D2, D2Array, D3, Cube, and CubeArray shapes; backend-wgpu proof is implemented for D2 and D2Array packed mip-chain transients.

Implemented evidence:
- `GraphTextureRendererDesc::rhi_height` now computes packed mip-chain backing height by summing each mip's flattened height.
- `RenderGraphBuilder::texture_desc_support` and `try_create_texture_from_desc` now accept mip-chain descriptors for supported one-sample graph-created texture dimensions.
- Full transient export promotion uses `read_rhi_packed_mip_chain_bytes` when the recorded renderer descriptor has more than one mip level, stores the promoted public texture as a packed mip-chain, and exposes complete public subresource metadata.
- The headless/RHI regression covers D1, D2, D2Array, D3, Cube, and CubeArray mip-chain descriptors in one test.
- The backend-wgpu regression covers graph-created D2 and D2Array packed mip-chain transients through real `Renderer::execute_graph_to_resources` on the wgpu backend.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_graph_created_mip_chain_texture_descs -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_promotes_graph_created_mip_chain_texture_desc -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_promotes_graph_created_d2_array_mip_chain_texture_desc -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed, 1 passed.

Remaining full-renderer scope:
- Native graph-created MSAA texture creation and resolve execution are implemented.
- Native surface/swapchain graph export promotion and persistent backend-resident graph resource synchronization remain open.


## 2026-05-20 - Graph-created MSAA transient descriptor and backend-wgpu resolve promotion

Area: RenderGraph API / RHI texture descriptors / MSAA graph-created transient promotion.

Status: Implemented for graph-created D2 MSAA transient texture creation, backend-wgpu RGBA8 resolve promotion, and renderer-level custom MSAA resolve validation.

Implemented evidence:
- `RhiTextureDesc` now carries `samples`, and headless/backend-wgpu RHI texture creation validates non-zero power-of-two sample counts.
- backend-wgpu RHI creates textures with the requested `sample_count` instead of always using one sample.
- `RenderGraphBuilder::texture_desc_support` and `try_create_texture_from_desc` now accept D2, one-mip graph-created MSAA descriptors and continue to reject unsupported MSAA shapes such as mipped MSAA descriptors.
- MSAA graph-created transient exports use `RENDER_ATTACHMENT` resolve semantics rather than invalid `COPY_SRC` usage on the multisampled source texture.
- `WgpuRhiDevice::read_texture_rgba8` resolves multisampled RGBA8 textures into a single-sample temporary texture before CPU readback.
- `Renderer::execute_graph_to_resources` promotes the resolved payload into a durable public texture while preserving `TextureInfo.samples` and `RendererGraphTextureExport.samples` as the original MSAA sample count.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_promotes_graph_created_msaa_texture_desc -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_try_create_texture_from_desc_validates_native_graph_shape -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer msaa -- --nocapture` passed, 7 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_resolves_msaa_texture_desc_with_custom_shader -- --nocapture` passed, 1 passed.

Remaining full-renderer scope:
- Native surface/swapchain graph export promotion and persistent backend-resident graph resource synchronization remain open.

## 2026-05-20 - RHI texture sample-count observability

Area: RHI / MSAA observability / backend parity.

Status: Implemented.

Implemented evidence:
- `RhiDevice::texture_samples(RhiTexture) -> Result<u32, RhiError>` is now part of the RHI public trait.
- `HeadlessRhiDevice` stores and reports the `RhiTextureDesc.samples` value for every RHI texture.
- `WgpuRhiDevice` stores and reports the native sample count used when creating backend-wgpu textures.
- This gives graph/MSAA tooling a real RHI evidence path for sample count instead of inferring sample metadata only from higher-level renderer descriptors.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_rhi_texture_samples_are_queryable -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_rhi_texture_samples_are_queryable -- --nocapture` passed, 1 passed.

Remaining full-renderer scope:
- Native surface/swapchain graph export promotion and persistent backend-resident graph resource synchronization remain open.

## 2026-05-20 - Persistent backend-wgpu graph texture import cache

Area: RenderGraph / backend-wgpu persistent resource synchronization.

Status: Superseded by the later texture+buffer import cache and destroy-time eviction implementation below.

Implemented evidence:
- `WgpuRendererRuntime` now owns a persistent `WgpuRhiDevice` and returns clones that share the same RHI state, instead of creating a fresh RHI state for every graph execution.
- `Renderer` now tracks a `graph_rhi_texture_import_cache` keyed by public `TextureHandle`.
- Public texture graph imports reuse the cached backend RHI texture when represented width/height/sample count/format and usage are compatible.
- When the public texture `revision` changes, the cache keeps the same backend RHI texture allocation and re-synchronizes bytes into it, then records the new revision.
- `Renderer::graph_rhi_texture_import_cache_entries()` and `Renderer::clear_graph_rhi_texture_import_cache()` expose a small public observability/control surface for the cache.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_reuses_persistent_texture_import_cache -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_ -- --nocapture` passed, 17 passed.

Remaining full-renderer scope:
- Readback-backed surface graph export promotion is implemented; direct swapchain image graph export remains a platform/wgpu capability-gated boundary.
- Custom resolve coverage is now represented in dedicated capability/query + execution rows.

## 2026-05-20 - Persistent backend-wgpu graph buffer import cache

Area: RenderGraph / backend-wgpu persistent resource synchronization.

Status: Implemented for public texture and buffer imports, including public-resource destroy-time cache eviction. Focused eviction coverage has passed in this pass.

Implemented evidence:
- `StoredBuffer` now carries a revision counter, matching texture-side synchronization semantics.
- `Renderer` now tracks `graph_rhi_buffer_import_cache` keyed by public `BufferHandle`.
- Public buffer graph imports reuse the cached backend RHI buffer when size and usage are compatible.
- When public buffer revision changes, the cache keeps the same backend RHI buffer allocation and re-synchronizes represented byte ranges into it, then records the new revision.
- `Renderer::graph_rhi_buffer_import_cache_entries()` and `Renderer::clear_graph_rhi_buffer_import_cache()` expose public observability/control for buffer import cache.
- `Renderer::destroy()` removes matching persistent graph RHI texture or buffer import cache entries when the public texture or buffer handle is destroyed.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_reuses_persistent_buffer_import_cache -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_reuses_persistent_texture_import_cache -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_ -- --nocapture` passed, 18 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer destroying_public_graph_import_resources_evicts_persistent_import_cache -- --nocapture` passed, 1 passed.

Remaining full-renderer scope:
- direct native surface/swapchain graph export capability gate promotion remains capability-gated beyond the implemented readback-backed and explicitly unsupported paths.
- Persistent backend-resident synchronization remains open. Current public custom resolve format coverage is closed by the later 2026-05-20 resolve entries.

## 2026-05-20 - Focused RenderGraph/RHI resolve stabilization

Area: RenderGraph / RHI MSAA resolve / backend-wgpu validation.

Status: Focused MSAA resolve and surface-export evidence passed; complete renderer goal remains open.

Implemented evidence:
- `GraphTextureRendererDesc` now preserves the source `TextureDesc::usage`, and graph-created RHI textures combine graph access-derived usage with descriptor usage. This lets graph-created resolve targets carry `STORAGE` or other required backend usage instead of relying only on pass declarations.
- Backend-wgpu graph execution submits already encoded graph command buffers before invoking callbacks that issue immediate RHI work. This makes previous graph writes visible to custom resolve callbacks in the same graph execution.
- Backend-wgpu RHI texture creation validates multisampled texture requirements before creating native textures: MSAA textures must be render attachments, unsupported copy/storage MSAA usage is rejected, and guaranteed format sample-count support is checked so invalid RGBA32F MSAA cases return validation or take the cap-gated test branch instead of panicking inside wgpu.
- Environment bake packed mip-chain textures clear stale base-level upload layout metadata when marking mips generated, preserving the packed mip-chain representation.
- Internally materialized public frame output textures no longer contribute to pending upload queue accounting, so frame-output readback placeholders do not pollute upload stats.
- The prelude boundary test now checks exact exported identifiers, and graph descriptor detail remains out of the game-layer prelude while public renderer-level graph export types remain available.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_resolves -- --nocapture` passed, 10 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_graphics_pipeline_sample_count -- --nocapture` passed, 2 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 4 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_ -- --nocapture` passed, 133 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu -- --test-threads=1` passed, 41 unit tests plus 1 integration test plus doc-tests.

Remaining full-renderer scope:
- direct native surface/swapchain graph export capability gate remains capability-gated; readback-backed promotion and unsupported provenance are implemented.
- Backend fence objects, true nonblocking per-submission completion queries, and remaining backend-owned tombstones remain open or partial in the matrix.

## 2026-05-20 - Depth32F custom MSAA resolve shaders

Area: RHI / RenderGraph MSAA resolve / backend-wgpu depth textures.

Status: Implemented for backend-wgpu Depth32F custom resolve and graph callback integration.

Implemented evidence:
- `RhiDevice::resolve_texture_depth32f_with_shader` exposes a backend-wgpu custom resolve path for multisampled Depth32Float textures.
- The shader ABI binds the source as `@group(0) @binding(0) texture_depth_multisampled_2d`; the caller fragment entry writes the resolved value through `@builtin(frag_depth)` into a single-sample `Depth32Float` render-attachment target.
- `PassContext::resolve_rhi_texture_depth32f_with_shader` exposes the same operation to RenderGraph callbacks.
- RHI graphics pipeline validation now allows depth-only fragment pipelines with no color target, matching the depth resolve and existing backend-wgpu depth-write implementation pattern.
- Headless RHI explicitly rejects the shader path as `UnsupportedFeature(BackendWgpu)`.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_resolves -- --nocapture` passed, 10 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 4 passed.

Remaining full-renderer scope:
- direct native surface/swapchain graph export capability gate remains capability-gated; readback-backed promotion and unsupported provenance are implemented.
- Backend fence objects, true nonblocking per-submission completion queries, and remaining backend-owned tombstones remain open or partial in the matrix.

## 2026-05-20 - 8-bit sRGB/BGRA custom MSAA resolve shaders

Area: RHI / RenderGraph MSAA resolve / backend-wgpu 8-bit color textures.

Status: Implemented for the current public 8-bit color `TextureFormat` custom resolve set. Future non-public or newly added formats remain future work.

Implemented evidence:
- `RhiDevice::resolve_texture_8bit_color_with_shader` exposes a backend-wgpu custom resolve path for multisampled `Rgba8UnormSrgb` and `Bgra8UnormSrgb` textures, alongside the existing public `Rgba8Unorm` custom resolve path.
- The shader ABI binds the source as `@group(0) @binding(0) texture_multisampled_2d<f32>` and expects the caller fragment entry to return the resolved color at `@location(0)`.
- Backend-wgpu writes the single-sample target through a color render attachment, avoiding storage-texture requirements for sRGB/BGRA target formats.
- `PassContext::resolve_rhi_texture_8bit_color_with_shader` exposes the same operation to RenderGraph callbacks.
- Headless RHI explicitly rejects the fragment shader path as `UnsupportedFeature(BackendWgpu)`.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer srgb_msaa_texture_with_custom_fragment_shader -- --nocapture` passed, 3 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer rhi_resolves -- --nocapture` passed, 10 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_pass_context_resolves -- --nocapture` passed, 4 passed.

Remaining full-renderer scope:
- direct native surface/swapchain graph export capability gate remains capability-gated; readback-backed promotion and unsupported provenance are implemented.
- Backend fence objects, true nonblocking per-submission completion queries, and remaining backend-owned tombstones remain open or partial in the matrix.

## 2026-05-20 coverage update: custom MSAA resolve capability query

Custom MSAA resolve is now represented by an explicit RHI support matrix instead of being inferred from method presence. Backend-wgpu supports RGBA8/RGBA16F/RGBA32F compute-storage custom resolves, 8-bit color fragment custom resolves, and Depth32Float fragment-depth custom resolves. Headless reports those WGSL custom resolve paths as unsupported with a user-visible reason. Graph passes can query the matrix through `PassContext::rhi_custom_resolve_support()` before selecting a path.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.




## 2026-05-20 coverage update: cooperative background retirement startup

Background resource retirement is no longer represented as an unsupported-only feature. The renderer can start/stop a cooperative retirement service state, performs an immediate retirement tick on start, and exposes active state in public memory and retirement stats. This closes the public API/startup/observability gap for background retirement while preserving the remaining external/architectural gap for a direct cross-thread renderer/wgpu mutation and nonblocking backend submission-index query.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer background_resource_retirement -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer renderer_feature -- --nocapture` passed, 4 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.


Implementation refinement: the background retirement service now has a real lightweight scheduler thread. It requests ticks atomically, and renderer safe points consume those requests on the main renderer thread. This closes worker start/stop lifecycle coverage while leaving true nonblocking backend completion queries as the remaining capability-gated gap.

## 2026-05-20 - Backend-wgpu native pipeline replacement tombstones

Area: backend-wgpu resource lifetime / native reflected pipeline cache.

Status: Implemented for native reflected pipeline object replacement; broader backend lifetime work remains partial.

Implemented evidence:
- `WgpuRendererRuntime::insert_native_pipeline_objects` now queues the previous native reflected pipeline object into backend-owned tombstones when an existing `PipelineKey` is replaced.
- The tombstoned object retains the old shader module, shader/layout objects, material bind groups, owned buffers, render-pipeline reference, and backend fence metadata until backend tombstone retirement.
- The structural native render-pipeline cache entry is removed only when no current native pipeline object still references it; replacement with the same structural render-pipeline key keeps the live cache entry available for the new object.
- `BackendResourceRetirementStats` reports live and retired counts for the replacement tombstone, including native pipeline entries, render-pipeline refs, shader modules, bind groups, owned buffers, and fence objects.

Validation:
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_native_pipeline_replacement_enters_backend_tombstone -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_native_cache_reuses_render_pipeline_across_material_bind_groups -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_wgpu::tests -- --test-threads=1` passed, 41 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

Remaining full-renderer scope:
- direct native surface/swapchain graph export capability gate remains capability-gated; readback-backed promotion and unsupported provenance are implemented.
- True backend fence objects/nonblocking per-submission completion queries and any backend-owned resource classes not yet represented by tombstones remain open or partial in the matrix.

## 2026-05-20 coverage update: pipeline cache backend-object coverage sync

Pipeline cache coverage now has a concrete artifact: `PipelineCacheBackendCoverage`. Facade cache entries synchronize `has_backend_object` against backend-wgpu native pipeline objects, and the coverage report lists missing backend-object keys instead of requiring tools to reinterpret aggregate counters. This improves the `CompleteBackendPipelineCache` gap from implicit Partial behavior to explicit per-key coverage.

Validation status: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer pipeline -- --nocapture` passed, 18 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

## 2026-05-20 coverage update: post-process backend coverage artifact

Post-process backend coverage now has a public artifact instead of relying on humans to compare `FramePostProcessOutput` against backend-native pass labels. `FramePostProcessBackendCoverage` lists covered and missing semantic post-process pass labels and preserves the backend labels that provided coverage. This tightens the Standard 3D / post-process matrix from broad Partial wording to per-frame evidence.

Validation status: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer post_process_backend_coverage -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

## 2026-05-20 coverage update: post-process support matrix

The post-process family now has a public support matrix. `PostProcessSupport` distinguishes facade-only renderers from backend-wgpu sampled-minimal implementations and lists the remaining production gap for each effect. This prevents the matrix from treating all post-process work as one ambiguous Partial bucket: backend visibility and production readiness are separate facts.

Validation status: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer post_process_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

## 2026-05-20 coverage update: deformation support matrix

Animation/deformation coverage now has a public matrix. `DeformationSupport` distinguishes facade-retained skeletal animation and morph targets, graph-observable LOD and motion-vector outputs, and the still-missing backend GPU deformation path. This prevents supported facade observability from being conflated with complete backend GPU skinning/morph execution.

Validation status: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer deformation_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

## 2026-05-20 coverage update: lighting and IBL support matrix

Light/shadow/environment coverage now has a public matrix. `RendererLightingSupport` distinguishes retained light descriptors, graph-observable shadows, retained environment IBL, backend IBL convolution, and runtime environment capture. This prevents retained/facade lighting observability from being conflated with complete backend-generated IBL probe convolution or capture.

Validation status: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer lighting_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

## 2026-05-20 coverage update: frame capture support matrix

Frame capture coverage now has a public support matrix. `FrameCaptureSupport` aggregates backend infos into internal availability, external-hook handoff coverage, native-SDK blocked backends, and unavailable backends. This makes the external SDK blocker explicit without hiding the implemented internal capture and callback handoff paths.

Validation status: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_capture_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

## 2026-05-20 coverage update: debug tooling support matrix

Debug/editor tooling now has a public matrix. `DebugToolingSupport` distinguishes retained debug draw commands, picking results, frame debug reports, frame capture, and native frame debugger capture. This prevents implemented renderer-side tooling from being conflated with the still-missing native RenderDoc/external-debugger SDK path.

Validation status: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer debug_tooling_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

## 2026-05-20 coverage update: resource lifecycle support matrix

Resource lifecycle coverage now has a public per-class matrix. `ResourceLifecycleSupport` distinguishes lifecycle/stale-handle coverage, upload/readback applicability, residency, observability, and backend residency level for renderer resource classes. Backend persistent synchronization gaps are now listed as explicit partial backend-residency facts instead of broad resource-layer ambiguity.

Validation status: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer resource_lifecycle_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

## 2026-05-20 coverage update: backend synchronization support matrix

Backend synchronization and retirement coverage now has a public matrix. `BackendSynchronizationSupport` distinguishes facade submission-boundary retirement, backend tombstone retirement, queue-empty fallback polling, true nonblocking submission-index polling, and the background retirement scheduler. The true nonblocking poll gap is now explicit and separately queryable from implemented fallback behavior.

Validation status: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_synchronization_support -- --nocapture` passed, 1 passed; `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

## 2026-05-20 coverage update: RenderGraph support matrix

`Renderer::render_graph_support()` is now part of the renderer coverage surface. It reports support entries for public buffer import/export, public D2 texture import/export, packed mip compatibility, flattened layer compatibility, graph-created D2 transient promotion, graph-created MSAA resolve promotion, custom MSAA resolve PassContext integration, persistent backend import cache, readback-backed surface graph export, and direct swapchain graph export.

Evidence added in this slice:

- `render_graph_support_reports_backend_and_swapchain_boundaries` was added to lock the public query shape and unsupported direct-swapchain boundary.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_support -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 tests plus doc-tests.

Status impact: this is observability and acceptance-boundary coverage, not renderer completion. Direct swapchain graph export, complete backend-resident synchronization, true nonblocking backend completion queries, and production-complete standard renderer paths remain open wherever the matrix still records `Partial`, `Stub`, `Missing`, unsupported-only, backend-incomplete, or support-matrix-only behavior.

## 2026-05-20 coverage update: backend material resource dependency invalidation

Resource lifecycle coverage now includes active invalidation for backend material-bound texture and sampler dependencies. `Renderer` resolves materials referencing a texture or sampler, unregisters backend-wgpu material external resource bindings for updated/destroyed texture or destroyed sampler handles, and invalidates native pipeline objects tagged with affected materials. Material parameter removal and full replacement now invalidate backend native pipeline objects as well.

Evidence added in this slice:

- `material_dependency_lookup_tracks_texture_and_sampler_users` was added for dependency lookup coverage across standard material texture fields and reflected material texture/sampler parameters.
- Covered by `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1`, which passed 408 tests plus doc-tests.

Status impact: texture/sampler/material backend binding lifecycle is stronger, but the resource lifecycle row remains partial until all backend-resident resource classes have complete dirty synchronization, upload/readback coherence, and retirement evidence.

## 2026-05-20 coverage update: backend material resource binding stats

Backend material resource lifecycle observability now includes `Renderer::backend_material_resource_stats()`. The report exposes backend-active state plus live material texture/sampler binding counts from backend-wgpu's material external resource registry. This complements the mutation/destroy invalidation path by giving tools a direct way to observe whether backend material bindings exist.

Evidence added in this slice:

- `WgpuMaterialExternalResourceStats` added for backend-wgpu registry counts.
- `BackendMaterialResourceStats` added to the public renderer facade and prelude.
- `backend_material_resource_stats_reports_headless_inactive` added for headless default behavior.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_material_resource_stats -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

Status impact: this improves lifecycle observability for material-bound backend resources. The broader resource lifecycle row remains partial until every backend-resident resource class has complete dirty synchronization and tested backend coherence.

## 2026-05-20 coverage update: backend material resource stats in frame/debug/capture

Backend material resource stats are now part of the standard observability surfaces. `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` expose `BackendMaterialResourceStats`, filled from `Renderer::backend_material_resource_stats()` during frame instrumentation.

Evidence added in this slice:

- `backend_material_resources` added to frame stats, debug report, capture, and resource dump structs.
- `frame_debug_report_preserves_backend_material_resource_stats` added for stats-to-debug propagation.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_preserves_backend_material_resource_stats -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

Status impact: material-bound backend texture/sampler residency is now observable from direct API, frame stats, debug reports, captures, and resource dumps. The broader renderer goal remains incomplete until every current matrix gap has real execution, errors, observability, tests, and synchronized docs.

## 2026-05-20 coverage update: material backend support matrix

Material backend coverage now has a public support matrix. `MaterialBackendSupport` distinguishes facade standard/custom material support, material template reflection validation, material info reflection diagnostics, backend-wgpu reflected custom material draws, backend texture/sampler material bindings, and the unsupported complete dynamic material-template backend pipeline path.

Evidence added in this slice:

- `MaterialBackendFeature`, `MaterialBackendImplementationLevel`, `MaterialBackendFeatureSupport`, and `MaterialBackendSupport` added to the public renderer facade and prelude.
- `Renderer::material_backend_support()` added as the renderer-facing query.
- `material_backend_support_distinguishes_facade_reflected_backend_and_dynamic_template_gap` added.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer material_backend_support -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

Status impact: Material API remains partial for complete dynamic material-template backend pipeline layout/bind-group integration, but reflected wgpu custom-material backend coverage and facade/schema diagnostics are now queryable instead of implicit.

## 2026-05-21 progress: material reflection coverage frame/capture observability

`MaterialReflectionCoverageStats` now promotes material-template/material reflection coverage from per-resource inspection into an aggregate renderer query and frame/capture diagnostic payload.

- Scope: Material API; Shader reflection diagnostics; Frame API / frame stats / frame capture.
- Implementation: `Renderer::material_reflection_coverage_stats()` summarizes Ready templates/materials, pipeline readiness, shader-interface/template readiness, schema/material reflection coverage, incomplete coverage counts, and missing reflected texture/sampler/buffer bindings.
- Frame/capture propagation: `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` now carry `material_reflection_coverage` from the same source of truth.
- Coverage hooks: `material_template_schema_is_validated_against_shader_reflection` asserts non-zero partial/full reflection aggregate counts, while `frame_debug_report_summarizes_last_frame_for_editor` and `material_backend_support_distinguishes_facade_reflected_backend_and_dynamic_template_gap` assert propagation into frame/debug/capture/resource-dump artifacts.
- Status impact: Material API remains Partial because complete dynamic material-template backend pipeline-layout/bind-group integration is still not implemented.

## 2026-05-21 progress: deformation support frame/capture observability

`DeformationSupport` now follows the same frame/capture observability path as other support matrices.

- Scope: Animation / skinning / morph / LOD; Frame API / frame stats / frame capture; editor/debug report.
- Implementation: `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` now carry `deformation_support` from `Renderer::deformation_support()`.
- Coverage hooks: `deformation_support_distinguishes_facade_outputs_from_backend_gpu_path` asserts query/frame/debug/capture/resource-dump consistency, and `frame_debug_report_summarizes_last_frame_for_editor` asserts report/capture propagation.
- Status impact: Animation / skinning / morph / LOD remains Partial because backend GPU skinning/morph/motion-vector shader-buffer execution is still not implemented.

## 2026-05-21 progress: lighting support frame/capture observability

`RendererLightingSupport` now follows the standard frame/capture support-matrix propagation path.

- Scope: Light / shadow / environment / IBL; Frame API / frame stats / frame capture; editor/debug report.
- Implementation: `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` now carry `lighting_support` from `Renderer::lighting_support()`.
- Coverage hooks: `lighting_support_distinguishes_retained_lighting_from_backend_ibl_convolution` asserts query/frame/debug/capture/resource-dump consistency, and `frame_debug_report_summarizes_last_frame_for_editor` asserts report/capture propagation.
- Status impact: Light / shadow / environment / IBL remains Partial because backend-real IBL convolution and runtime environment capture are still not implemented.

## 2026-05-21 progress: resource lifecycle support frame/capture observability

`ResourceLifecycleSupport` now follows the standard frame/capture support-matrix propagation path.

- Scope: Resource status/destroy; Frame API / frame stats / frame capture; editor/debug report.
- Implementation: `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` now carry `resource_lifecycle_support` from `Renderer::resource_lifecycle_support()`.
- Coverage hooks: `resource_lifecycle_support_reports_per_class_lifecycle_and_backend_gaps` asserts query/frame/debug/capture/resource-dump consistency, and `frame_debug_report_summarizes_last_frame_for_editor` asserts report/capture propagation.
- Status impact: Resource status/destroy remains Partial because per-resource backend fence objects and persistent backend lifetime objects are still not represented as independent fence-backed resources.

## 2026-05-21 progress: backend synchronization support frame/capture observability

`BackendSynchronizationSupport` now follows the standard frame/capture support-matrix propagation path.

- Scope: Resource status/destroy; GPU memory/upload/streaming; backend submission/retirement observability; Frame API / frame stats / frame capture.
- Implementation: `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` now carry `backend_synchronization_support` from `Renderer::backend_synchronization_support()`.
- Coverage hooks: `backend_synchronization_support_reports_polling_and_scheduler_limits` asserts query/frame/debug/capture/resource-dump consistency, and `frame_debug_report_summarizes_last_frame_for_editor` asserts report/capture propagation.
- Status impact: Resource status/destroy and GPU memory/upload/streaming remain Partial because independent per-resource backend fence objects and tombstones for remaining backend-owned classes are still not complete.

## 2026-05-21 progress: post-process support frame/capture observability

`PostProcessSupport` now follows the standard frame/capture support-matrix propagation path.

- Scope: Standard 3D graph; post-process backend coverage; Frame API / frame stats / frame capture.
- Implementation: `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` now carry `post_process_support` from `Renderer::post_process_support()`.
- Coverage hooks: `post_process_support_distinguishes_backend_visible_from_production_ready` asserts query/frame/debug/capture/resource-dump consistency, and `frame_debug_report_summarizes_last_frame_for_editor` asserts report/capture propagation.
- Status impact: Standard 3D graph remains Partial because backend-visible sampled post-process branches are not production-ready post-process resource chains.

## 2026-05-22 progress: frame capture support frame/capture observability

`FrameCaptureSupport` now follows the standard frame/capture support-matrix propagation path.

- Scope: Frame API / frame stats / frame capture; Frame capture / RenderDoc hooks; editor/debug report.
- Implementation: `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` now carry `frame_capture_support` from `Renderer::frame_capture_support()`.
- Coverage hooks: `frame_capture_support_distinguishes_internal_hooks_and_native_sdk_blockers` asserts query/frame/debug/capture/resource-dump consistency, and `frame_debug_report_summarizes_last_frame_for_editor` asserts report/capture propagation.
- Status impact: Frame capture / RenderDoc hooks remains External Blocked for built-in RenderDoc/external-debugger SDK loading and capture begin/end calls; current code provides internal capture plus registered callback handoff.

## 2026-05-22 progress: debug tooling support frame/capture observability

`DebugToolingSupport` now follows the standard frame/capture support-matrix propagation path.

- Scope: Debug draw/editor API; Frame API / frame stats / frame capture; editor/debug report.
- Implementation: `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` now carry `debug_tooling_support` from `Renderer::debug_tooling_support()`.
- Coverage hooks: `debug_tooling_support_keeps_native_debugger_sdk_blocker_explicit` asserts query/frame/debug/capture/resource-dump consistency, and `frame_debug_report_summarizes_last_frame_for_editor` asserts report/capture propagation.
- Status impact: Debug draw/editor API remains Implemented at the renderer layer; native frame debugger SDK capture remains represented as a blocker in the support matrix rather than as a renderer debug-draw/editor implementation gap.

## 2026-05-20 coverage update: graph RHI import cache dirty-state stats

Persistent graph RHI import cache coverage now includes dirty-state observability. `Renderer::graph_rhi_import_cache_stats()` reports texture/buffer cache entry counts and stale public-resource revision counts through `RendererGraphRhiImportCacheStats`. This records whether cached graph imports are synchronized with current public buffer/texture revisions or waiting for the next import path to update the cached RHI resource.

Evidence added in this slice:

- `RendererGraphRhiImportCacheStats` added to the public API and prelude.
- `graph_rhi_import_cache_stats_reports_stale_public_revisions` added for stale revision reporting.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_rhi_import_cache_stats -- --nocapture` passed, 2 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

Status impact: graph import cache dirty-sync is now observable instead of implicit. The full renderer goal remains incomplete until every graph/backend/resource lifecycle gap has real execution, error handling, observability, tests, and synchronized documentation.

## 2026-05-20 coverage update: graph RHI import cache stats in frame/debug/capture

Graph RHI import cache dirty-state stats are now available through standard observability surfaces. `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` include `RendererGraphRhiImportCacheStats`, populated during frame instrumentation from `Renderer::graph_rhi_import_cache_stats()`.

Evidence added in this slice:

- `graph_rhi_import_cache` added to frame stats, debug report, capture, and resource dump structs.
- `frame_debug_report_preserves_graph_rhi_import_cache_stats` added for propagation coverage.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_preserves_graph_rhi_import_cache_stats -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_rhi_import_cache_stats -- --nocapture` passed, 2 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

Status impact: persistent graph import cache dirty revisions are observable from direct API, frame stats, debug reports, captures, and resource dumps. The renderer goal remains incomplete until remaining backend/resource/standard-renderer gaps have real execution, errors, observability, tests, and synchronized docs.

## 2026-05-20 coverage update: graph import cache stale byte/range accounting

Graph RHI import cache dirty-state coverage now includes synchronization footprint accounting. `RendererGraphRhiImportCacheStats` reports stale texture bytes, stale buffer represented range counts, stale buffer bytes, and aggregate stale bytes in addition to entry counts.

Evidence added in this slice:

- `RendererGraphRhiImportCacheStats` extended with stale byte/range fields.
- `graph_rhi_import_cache_stats_reports_stale_public_revisions` updated to assert the dirty footprint after public texture and buffer updates.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_rhi_import_cache_stats -- --nocapture` passed, 2 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

Status impact: persistent graph import cache dirty work is now quantifiable through direct API and the frame/debug/capture propagation added earlier. Broader backend residency and standard renderer completeness remain open.

## 2026-05-20 coverage update: pipeline cache backend coverage in frame/debug/capture

Pipeline cache facade/backend object coverage is now part of frame observability. `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` include `PipelineCacheBackendCoverage`, populated during frame instrumentation from `Renderer::pipeline_cache_backend_coverage()`.

Evidence added in this slice:

- `pipeline_cache_backend_coverage` added to frame stats, debug report, capture, and resource dump structs.
- `frame_debug_report_preserves_pipeline_cache_backend_coverage` added for propagation coverage.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_preserves_pipeline_cache_backend_coverage -- --nocapture` passed, 1 passed.

Status impact: backend pipeline object coverage is now visible from direct API and standard stats/debug/capture outputs. `RendererFeature::CompleteBackendPipelineCache` remains incomplete until all relevant facade pipeline entries are backend-backed in real rendering paths.

## 2026-05-20 coverage update: pipeline cache backend coverage missing-entry classification

Pipeline cache backend coverage now separates missing backend object entries into ready missing entries, used missing entries, and unused missing entries. This improves diagnosis of `CompleteBackendPipelineCache` gaps in direct API, frame stats, debug reports, captures, and resource dumps.

Evidence added in this slice:

- `PipelineCacheBackendCoverage` extended with `ready_missing_backend_object_entries` and `unused_missing_backend_object_entries`.
- Focused coverage expectations updated for the missing-backend-object case.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer pipeline_warmup_validates_pipeline_keys -- --nocapture` passed, 1 passed.

Status impact: backend pipeline cache gaps are more precisely observable. The renderer goal remains incomplete until real rendering paths produce complete backend object coverage where required.

## 2026-05-20 coverage update: sampler info and destroyed texture-view output coverage

The texture/sampler row now has explicit sampler descriptor/status inspection through `SamplerInfo` and `Renderer::sampler_info()`. Sampler info returns live retained descriptors, while destroyed sampler payloads continue to be observed through `Renderer::resource_status()` as `DestroyQueued`. Texture-view public frame output coverage now includes a destroyed-target test, so stale texture-view frame targets cannot silently produce public frame-output bytes.

Evidence added in this slice:

- `SamplerInfo` added to the public renderer facade and prelude.
- `Renderer::sampler_info()` added for sampler descriptor/status inspection.
- `sampler_info_reports_desc_status_and_destroyed_payload_boundary` added.
- `texture_view_frame_output_rejects_destroyed_target_texture` added.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer sampler_info_reports_desc_status_and_destroyed_payload_boundary -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer texture_view_frame_output_rejects_destroyed_target_texture -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

Status impact: texture/sampler API coverage is tighter and a specialized destroyed texture-view frame-output path is now tested. The row remains `Partial` because public generated mip data is still retained for headless/tooling compatibility and backend GPU mip generation remains limited to the current layer-based filterable sampled material texture path.

## 2026-05-20 coverage update: backend submission completion report

Backend synchronization coverage now includes `BackendSubmissionCompletionReport`. The report is available from `Renderer::backend_submission_completion_report()` and from frame/debug/capture outputs. It distinguishes inactive backend state, queue-empty fallback polling, recorded submission-index availability, and the availability of true nonblocking per-submission completion when runtime polling observes an active completion tracker.

Evidence added in this slice:

- Public report type and renderer query added.
- `FrameStats`, `FrameDebugReport`, and `FrameCapture` now carry the report.
- `backend_submission_completion_report_exposes_nonblocking_limit` and `frame_debug_report_preserves_backend_submission_completion_report` added.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `backend_submission_completion_report_exposes_nonblocking_limit` and `frame_debug_report_preserves_backend_submission_completion_report`.

Status impact: backend completion behavior is now explicit and observable. `NonblockingResourceRetirementPoll` is conditionally complete: when `WgpuRendererRuntime` currently tracks an active true completion object the poll path can succeed, and the API reports unsupported when only queue-empty fallback is available.

## 2026-05-20 coverage update: backend submission completion in resource dumps

Backend submission completion state is now included in resource dumps as well as frame stats, debug reports, and captures. `FrameCaptureResourceDump::backend_submission_completion` preserves the same `BackendSubmissionCompletionReport` fields used elsewhere.

Evidence added in this slice:

- `backend_submission_completion` added to `FrameCaptureResourceDump`.
- Existing focused propagation test extended to cover resource dumps.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `frame_debug_report_preserves_backend_submission_completion_report`.

Status impact: backend completion observability is now consistently available across direct query, stats, debug report, capture, and resource dump outputs. The implementation reports true nonblocking per-submission completion when a tracker exists, and queue-empty fallback remains the safe backstop when it does not.

## 2026-05-20 coverage update: backend submission completion in retirement stats

Backend submission completion state is now part of `ResourceRetirementStats`. `Renderer::poll_resource_retirements()` reports the same `BackendSubmissionCompletionReport` propagated through frame stats, debug reports, captures, and resource dumps.

Evidence added in this slice:

- `backend_submission_completion` added to `ResourceRetirementStats`.
- `resource_retirement_stats_preserve_backend_submission_completion_report` added.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `resource_retirement_stats_preserve_backend_submission_completion_report`.

Status impact: resource-retirement API observability is aligned with the rest of the renderer. The implementation reports queue-empty fallback as a safe backstop and reports active true nonblocking completion only while completion trackers are live.

## 2026-05-20 coverage update: backend completion report tombstone counters

Backend submission completion reporting now includes tombstone wait/retire counters. `BackendSubmissionCompletionReport` exposes pending tombstones, tombstones waiting for submission-index retirement, tombstones waiting for queue-empty fallback retirement, retired tombstones in the last poll, and the retirement mode used by the last poll.

Evidence added in this slice:

- `BackendSubmissionCompletionReport` extended with tombstone pressure fields.
- Renderer report construction now maps those fields from backend retirement stats.
- Focused completion-report expectations updated.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `backend_submission_completion_report_exposes_nonblocking_limit`.

Status impact: backend completion and backend resource retirement are now observable through one report. Queue-empty fallback is still the safe backstop, while true nonblocking per-submission completion is now reported when a tracker is active.

## 2026-05-20 coverage update: external render target destroyed attachment validation

External render target descriptors are revalidated during frame rendering, including attachment liveness. The focused coverage destroys color and depth attachments after creating external render targets and verifies that `Frame::render_view()` rejects each stale attachment with a texture invalid-handle error while the external render target handle itself remains inspectable.

Evidence added in this slice:

- `external_render_target_rejects_destroyed_attachment_at_frame_time` added.
- `external_render_target_rejects_destroyed_depth_attachment_at_frame_time` added.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer external_render_target_rejects_destroyed_attachment_at_frame_time -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer external_render_target_rejects_destroyed_depth_attachment_at_frame_time -- --nocapture` passed, 1 passed.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests.

Status impact: one more specialized destroyed frame-output path is now covered. Broader resource/backend completion remains open where the matrix still records `Partial` rows.

## 2026-05-20 coverage update: render view target rejection for destroyed plain textures

`Frame::render_view()` now has explicit coverage for destroyed plain texture render targets, covering the direct-`RenderTarget::Texture` variant (not only `RenderTarget::TextureView` / external target attachments). The focused regression destroys a render-target texture before frame execution and verifies that the view submission returns `RendererError::InvalidHandle` and the destroyed handle stays queued for delayed reclaim.

Evidence added in this slice:

- `render_view_rejects_destroyed_texture_target` added.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_view_rejects_destroyed_texture_target -- --nocapture` passed, 1 passed.

Status impact: texture output frame-path stale-handle rejection coverage is broader for texture render target variants. Broader render target and resource lifecycle completeness is still open where matrix rows remain `Partial`.

## 2026-05-20 coverage update: build_view_graph_stats rejects destroyed plain textures

The graph prebuild validation path now rejects destroyed plain texture render targets before graph execution planning. The focused regression destroys a `RenderTarget::Texture` handle and verifies that `build_view_graph_stats(...)` returns `RendererError::InvalidHandle` for the destroyed texture handle.

Evidence added in this slice:

- `build_view_graph_stats_rejects_destroyed_texture_target` added.

## 2026-05-20 coverage update: build_view_graph_stats rejects destroyed texture-view targets

The graph prebuild validation path now also rejects destroyed texture-view render targets. The focused regression destroys a texture handle and then validates a `RenderTarget::TextureView` view descriptor through `build_view_graph_stats(...)`, expecting `RendererError::InvalidHandle` with the original texture handle.

Evidence added in this slice:

- `build_view_graph_stats_rejects_destroyed_texture_view_target` added.

## 2026-05-20 coverage update: explicit nonblocking backend completion error path

Backend synchronization coverage now includes a callable nonblocking completion gate. `Renderer::poll_backend_submission_completion_nonblocking()` returns a user-visible validation error when true nonblocking per-submission backend completion polling is unavailable, rather than requiring callers to infer that state from reports alone.

Evidence added in this slice:

- Public nonblocking completion poll API added.
- Focused user-visible error test added for headless and backend-wgpu front-loaded no-tracker paths.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `nonblocking_backend_submission_completion_poll_reports_user_visible_error`.
- Added focused nonblocking success-path test; `cargo test -p engine_renderer nonblocking_backend_submission_completion_poll_can_be_supported_after_real_submission -- --test-threads=1` added and runs to confirm successful completion-path reporting when a tracked runtime submission exists.
- Added focused no-tracker regression for backend-wgpu paths where queue-empty polling is not a true nonblocking completion path; `cargo test -p engine_renderer nonblocking_backend_submission_completion_poll_reports_user_visible_error_without_trackers -- --test-threads=1` added and confirms error path remains when no completion tracker exists.
- Added focused tracker-drain fallback regression: `cargo test -p engine_renderer nonblocking_backend_submission_completion_poll_can_fall_back_when_trackers_drain -- --test-threads=1` added and confirms the true nonblocking completion path transitions false once completion trackers are no longer active.
- Added focused support-matrix transition regression to prove `BackendSynchronizationSupport` flips from fallback-only to true nonblocking coverage when a tracked completion object is observed; `cargo test -p engine_renderer backend_synchronization_support_reports_true_nonblocking_after_tracked_completion -- --test-threads=1` added and confirms `unsupported_features()` becomes empty in that state.
- Added focused backend-wgpu tracker-reuse regression to ensure repeated tombstones that share one `submission_index` reuse the same tracker order and do not register duplicate completion trackers; `cargo test -p engine_renderer wgpu_submission_fence_reuses_tracker_for_repeated_same_submission_index -- --test-threads=1` added.

Status impact: `NonblockingResourceRetirementPoll` now reflects active tracker presence more accurately; it can return success only when an active true nonblocking completion tracker is currently tracked, and can fall back once trackers drain. The overall backend goal remains incomplete where no tracker exists yet or where broader backend resident/retirement paths remain queue-fallback-only.

## 2026-05-20 coverage update: explicit direct swapchain graph export gate

Direct swapchain graph export coverage now includes a callable public gate. `Renderer::require_direct_swapchain_graph_export_supported()` returns a user-visible validation error on current renderer paths, using the same limitation text as `surface_graph_export_support()`.

Evidence added in this slice:

- Public direct-swapchain graph export gate added.
- Focused user-visible error test added.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `direct_swapchain_graph_export_gate_returns_user_visible_error`.

Status impact: direct swapchain graph export has explicit public error semantics, but remains incomplete because native swapchain image export/promotion is still not implemented.

## 2026-05-20 coverage update: surface graph export support in frame/debug/capture

Surface graph export support is now included in standard observability outputs. `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` carry `RendererSurfaceGraphExportSupport`, populated during frame instrumentation from `Renderer::surface_graph_export_support()`.

Evidence added in this slice:

- `surface_graph_export` added to frame stats, debug report, capture, and resource dump structs.
- `frame_debug_report_preserves_surface_graph_export_support` added for propagation coverage.
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer -- --test-threads=1` passed, 408 passed plus doc-tests, including `frame_debug_report_preserves_surface_graph_export_support`.

Status impact: direct/readback-backed surface graph export capability state is now visible from direct API, frame stats, debug reports, captures, and resource dumps. Native direct swapchain graph export remains incomplete.

## 2026-05-20 coverage update: RenderGraph support matrix in frame/debug/capture

RenderGraph support coverage is now included in standard observability outputs. `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` carry `RendererRenderGraphSupport`, populated during frame instrumentation from `Renderer::render_graph_support()`.

Evidence added in this slice:

- `render_graph_support` added to frame stats, debug report, capture, and resource dump structs.
- `RendererRenderGraphSupport` made default-constructible.
- `frame_debug_report_preserves_render_graph_support_matrix` added for propagation coverage.
- `cargo test -p engine_renderer frame_debug_report_preserves_render_graph_support_matrix -- --nocapture` passed.

Status impact: RenderGraph capability support, including unsupported direct swapchain graph export, is visible from direct API, frame stats, debug reports, captures, and resource dumps. Native direct swapchain graph export remains incomplete.

## 本轮 Surface 句柄校验证据补充

- 能力项：窗口/Surface 集成与错误语义。
- 本轮实现：新增 `with_surface_validates_display_handles_for_surface_creation` 回归测试（仅在 `backend-wgpu` feature 下运行），通过 `DummySurfaceWindowWithoutDisplay` 覆盖 `HasWindowHandle` 有效但 `HasDisplayHandle` 不可用的窗口场景。
- 闭合语义：`Renderer::with_surface` 在创建前会校验 display 句柄；当 `HasDisplayHandle` 返回 `Err(HandleError::Unavailable)` 时，直接返回 `RendererError::Validation`，并在错误文本中包含 `display`，形成与窗口句柄无效分支独立的验证覆盖。
- 验证命令：`cargo test -p engine_renderer with_surface_validates_display_handles_for_surface_creation -- --nocapture`
- 验证结果：1 passed。
- 剩余状态：该窗口/Surface 创建路径的错误语义覆盖继续按现有策略补充更多边界分支（如窗口句柄无效、已销毁窗口）

- 额外收敛（短路验证）：新增 `with_surface_short_circuits_display_validation_on_window_handle_error` 回归。
- 能力项：窗口/Surface 初始化语义与错误短路路径。
- 本轮实现：新增 `DummySurfaceWindowInvalidWindowCallsDisplay` 桩，使用 `AtomicBool` 记录 `display_handle()` 是否被误触发；窗口句柄返回 `Unavailable` 时若进入 display 分支将 panic。
- 闭合语义：窗口句柄无效会直接返回验证错误，不会继续执行 display 校验；避免将窗口问题误报为 display 问题。
- 验证命令：`cargo test -p engine_renderer with_surface_short_circuits_display_validation_on_window_handle_error -- --nocapture`
- 验证结果：1 passed。

- 同步补充：新增 `with_surface_invokes_window_handle_validation_before_display_validation` 回归。
- 能力项：窗口/Surface 句柄校验顺序。
- 本轮实现：新增带顺序计数的桩对象 `DummySurfaceWindowWithDisplayValidationCounter`，校验窗口句柄先于 display 句柄调用，并且两者均只调用一次。
- 验证命令：`cargo test -p engine_renderer with_surface_invokes_window_handle_validation_before_display_validation -- --nocapture`
- 验证结果：1 passed。

## 2026-05-21 coverage update: frame capture external backend without callback

外部 capture 后端在只通过 hook 注册（未注册 callback）时，仍会在 `pending` 与 `finish` 期间报告 `BackendHookRequested` handoff，并保持 `external_hook_triggered` 与 hook 元数据传播。该覆盖把无 callback 的外部 hook 注册路径单独固定下来，避免与 callback 成功/失败路径混淆。

Evidence added in this slice:

- `frame_capture_external_backend_available_without_callback_reports_hook_requested` added.
- `cargo test -p engine_renderer frame_capture_external_backend_available_without_callback_reports_hook_requested -- --nocapture` passed.

## 2026-05-21 coverage update: stale resource handles reject by generation and keep kind tags

同一 `index`、不同 `generation` 的同种类资源句柄应当表现一致为失效。新增回归将一组实际可用资源（`Mesh/Buffer/Texture/Sampler/Shader/MaterialTemplate/Material/Environment/Skeleton/MorphWeights/Scene/Camera/RenderGraphExtension`）转换为 generation 不匹配的 `stale` 句柄，验证公共错误路径在 `resource_status`、`destroy`、`set_resource_priority` 上保持 `ResourceKind` 标记化错误语义。

- 本轮实现：新增 `generic_resource_generation_mismatch_returns_invalid_handle_for_stale_handles`，分别覆盖上述资源在 `generation` 偏移后的 `InvalidHandle` 行为一致性。
- 验证命令：`cargo test -p engine_renderer generic_resource_generation_mismatch_returns_invalid_handle_for_stale_handles -- --nocapture`
- 验证结果：1 passed。

## 2026-05-21 coverage update: mesh and buffer update API reject stale generation handles

Mesh 与 Buffer 的更新接口在 handle 过期语义上与公共资源错误模型保持一致：对同 `index` 的 `generation+1` 旧句柄，应当立刻拒绝为 `InvalidHandle`，不触发更新执行。

- 本轮实现：新增 `buffer_update_rejects_stale_generation_handles` 和 `mesh_update_rejects_stale_generation_handles`，分别覆盖 `update_buffer` 与 `update_mesh_vertices`/`update_mesh_indices` 的 stale generation 路径。
- 验证命令：`cargo test -p engine_renderer buffer_update_rejects_stale_generation_handles -- --nocapture`
- 验证结果：未在本次提交中执行。
- 验证命令：`cargo test -p engine_renderer mesh_update_rejects_stale_generation_handles -- --nocapture`
- 验证结果：未在本次提交中执行。

## 2026-05-21 coverage update: texture update/generate_mips reject stale generation handles

Texture 的更新与 mip 路径也要保持同一 `InvalidHandle` 语义：同 `index` 的 `generation+1` 旧句柄在 `update_texture` 与 `generate_mips` 上必须立即被拒绝，并保留 `ResourceKind::Texture` 标记和 raw 值。

- 本轮实现：新增 `texture_update_and_generate_mips_reject_stale_generation_handles`，覆盖 `TextureUsage::COPY_DST` 更新和 `generate_mips` 两类 stale 句柄行为。
- 验证命令：`cargo test -p engine_renderer texture_update_and_generate_mips_reject_stale_generation_handles -- --nocapture`
- 验证结果：未在本次提交中执行。

## 2026-05-21 coverage update: stale handle query APIs return None

`buffer_info`/`buffer_bytes`、`texture_info`/`texture_bytes`、`sampler_info` 这些查询接口在 `generation+1` 过期句柄上应走一致的非暴露语义，返回 `None` 而不是伪造 payload。

- 本轮实现：新增 `info_queries_return_none_for_stale_generation_handles`，覆盖 buffer、texture、sampler 的查询面板。
- 验证命令：`cargo test -p engine_renderer info_queries_return_none_for_stale_generation_handles -- --nocapture`
- 验证结果：未在本次提交中执行。

## 2026-05-21 coverage update: texture mutation rejects destroyed handles

销毁后的纹理不应允许继续执行 mutation 路径。该回归覆盖 `update_texture` 与 `generate_mips` 在 `DestroyQueued` 纹理句柄上的错误语义，要求返回明确的 `RendererError::InvalidHandle`，并与现有 `resource_status` / 查询行为一致。

- 本轮实现：新增 `texture_mutation_rejects_destroyed_handle`。
- 验证命令：`cargo test -p engine_renderer texture_mutation_rejects_destroyed_handle -- --nocapture`
- 验证结果：未在本次提交中执行。

## 2026-05-21 coverage update: texture query API returns None after destroy

`texture_info` 与 `texture_bytes` 在销毁后的 `DestroyQueued` 纹理句柄上应与资源失效语义保持一致，返回 `None`，并且继续通过 `resource_status` 暴露 `DestroyQueued` 载荷用于寿命观察。

- 本轮实现：新增 `texture_queries_return_none_for_destroyed_handle`。
- 验证命令：`cargo test -p engine_renderer texture_queries_return_none_for_destroyed_handle -- --nocapture`
- 验证结果：未在本次提交中执行。
