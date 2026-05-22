use engine_asset::prelude::*;

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
}

fn server_with_textures(paths: &[(&str, u8)]) -> AssetServer {
    let mut io = MemoryAssetIo::new();
    for (path, value) in paths {
        io.insert(*path, texture_bytes(1, 1, *value));
    }
    let mut server = AssetServer::new(AssetServerConfig {
        max_io_jobs_per_frame: 1,
        ..AssetServerConfig::default()
    });
    server.set_io(io);
    server.register_builtin_loaders();
    server
}

fn server_with_textures_and_config(paths: &[(&str, u8)], config: AssetServerConfig) -> AssetServer {
    let mut io = MemoryAssetIo::new();
    for (path, value) in paths {
        io.insert(*path, texture_bytes(1, 1, *value));
    }
    let mut server = AssetServer::new(config);
    server.set_io(io);
    server.register_builtin_loaders();
    server
}

fn finish_uploads(server: &mut AssetServer) {
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(1))),
    );
}

#[cfg(feature = "async_loading")]
fn update_until_state(
    server: &mut AssetServer,
    id: AssetId,
    expected: AssetLoadState,
) -> AssetLoadState {
    for _ in 0..50 {
        server.update_loading();
        let state = server.state_by_id(id);
        if state == expected {
            return state;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    server.state_by_id(id)
}

#[test]
fn cancel_load_by_id_removes_queued_request_and_emits_event() {
    let mut server = server_with_textures(&[("textures/a.texture", 1)]);
    let texture: Handle<Texture> = server.load("textures/a.texture");

    assert_eq!(server.state(&texture), AssetLoadState::Queued);
    assert!(server.cancel_load_by_id(texture.id()));
    assert_eq!(server.state(&texture), AssetLoadState::Cancelled);
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Cancelled { id } if *id == texture.id())));

    server.update_loading();
    assert_eq!(server.state(&texture), AssetLoadState::Cancelled);
    assert!(server.drain_gpu_uploads().next().is_none());
    assert!(server
        .dependency_graph()
        .direct_dependencies(texture.id())
        .is_empty());
}

#[test]
fn cancel_load_by_path_allows_later_requeue() {
    let mut server = server_with_textures(&[("textures/a.texture", 2)]);
    let path = AssetPath::parse("textures/a.texture");
    let first: Handle<Texture> = server.load(path.clone());

    assert!(server.cancel_load_by_path(path.clone()));
    assert_eq!(server.state(&first), AssetLoadState::Cancelled);

    let second: Handle<Texture> = server.load(path);
    assert_eq!(second.id(), first.id());
    assert_eq!(server.state(&second), AssetLoadState::Queued);
    server.update_loading();
    finish_uploads(&mut server);

    assert_eq!(server.state(&second), AssetLoadState::Ready);
    assert_eq!(server.get(&second).unwrap().width, 1);
}

#[test]
fn cancel_load_group_marks_all_queued_group_assets_cancelled() {
    let mut server = server_with_textures(&[
        ("textures/a.texture", 3),
        ("textures/b.texture", 4),
        ("textures/c.texture", 5),
    ]);
    let group = server.load_group(&[
        AssetPath::parse("textures/a.texture"),
        AssetPath::parse("textures/b.texture"),
        AssetPath::parse("textures/c.texture"),
    ]);

    assert_eq!(server.cancel_load_group(&group), 3);
    assert_eq!(server.group_state(&group), AssetLoadState::Cancelled);
    for handle in &group.assets {
        assert_eq!(server.state_by_id(handle.id()), AssetLoadState::Cancelled);
    }

    server.update_loading();
    assert!(server.drain_gpu_uploads().next().is_none());
}

#[test]
fn scheduler_priority_and_dedup_survive_cancellation() {
    let mut server =
        server_with_textures(&[("textures/low.texture", 6), ("textures/high.texture", 7)]);
    let low: Handle<Texture> = server.load_with_priority("textures/low.texture", LoadPriority::Low);
    let duplicate_low: Handle<Texture> =
        server.load_with_priority("textures/low.texture", LoadPriority::Immediate);
    let high: Handle<Texture> =
        server.load_with_priority("textures/high.texture", LoadPriority::High);

    assert_eq!(low.id(), duplicate_low.id());
    assert!(server.cancel_load_by_id(low.id()));
    server.update_loading();

    assert_eq!(server.state(&low), AssetLoadState::Cancelled);
    assert_eq!(server.state(&high), AssetLoadState::UploadingGpu);
    finish_uploads(&mut server);
    assert_eq!(server.state(&high), AssetLoadState::Ready);
}

#[test]
fn update_loading_respects_cpu_job_budget_as_well_as_io_budget() {
    let mut server = server_with_textures_and_config(
        &[
            ("textures/a.texture", 8),
            ("textures/b.texture", 9),
            ("textures/c.texture", 10),
        ],
        AssetServerConfig {
            max_io_jobs_per_frame: 64,
            max_cpu_jobs_per_frame: 1,
            ..AssetServerConfig::default()
        },
    );
    let a: Handle<Texture> = server.load("textures/a.texture");
    let b: Handle<Texture> = server.load("textures/b.texture");
    let c: Handle<Texture> = server.load("textures/c.texture");

    let report = server.loading_policy_report();
    assert_eq!(report.effective_jobs_per_frame, 1);

    server.update_loading();
    assert_eq!(server.state(&a), AssetLoadState::UploadingGpu);
    assert_eq!(server.state(&b), AssetLoadState::Queued);
    assert_eq!(server.state(&c), AssetLoadState::Queued);

    finish_uploads(&mut server);
    server.update_loading();
    assert_eq!(server.state(&a), AssetLoadState::Ready);
    assert_eq!(server.state(&b), AssetLoadState::UploadingGpu);
    assert_eq!(server.state(&c), AssetLoadState::Queued);
}

#[cfg(feature = "async_loading")]
#[test]
fn async_loading_dispatches_worker_and_collects_result_on_later_update() {
    let mut server = server_with_textures_and_config(
        &[("textures/async.texture", 11)],
        AssetServerConfig {
            enable_async_loading: true,
            max_io_jobs_per_frame: 1,
            max_cpu_jobs_per_frame: 1,
            ..AssetServerConfig::default()
        },
    );
    let texture: Handle<Texture> = server.load("textures/async.texture");

    server.update_loading();
    assert_eq!(server.state(&texture), AssetLoadState::LoadingBytes);

    assert_eq!(
        update_until_state(&mut server, texture.id(), AssetLoadState::UploadingGpu),
        AssetLoadState::UploadingGpu
    );
    finish_uploads(&mut server);
    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!(server.get(&texture).unwrap().width, 1);
}

#[cfg(feature = "async_loading")]
#[test]
fn async_worker_pool_reuses_thread_and_reports_shutdown() {
    let mut server = server_with_textures_and_config(
        &[
            ("textures/pool_a.texture", 21),
            ("textures/pool_b.texture", 22),
        ],
        AssetServerConfig {
            enable_async_loading: true,
            max_io_jobs_per_frame: 1,
            max_cpu_jobs_per_frame: 1,
            ..AssetServerConfig::default()
        },
    );
    let first: Handle<Texture> = server.load("textures/pool_a.texture");

    server.update_loading();
    let first_report = server.async_worker_pool_report();
    assert!(first_report.enabled);
    assert_eq!(first_report.desired_workers, 1);
    assert_eq!(first_report.active_workers, 1);
    assert_eq!(first_report.in_flight_jobs, 1);
    assert_eq!(first_report.dispatched_jobs, 1);
    assert_eq!(first_report.worker_threads_started, 1);
    assert_eq!(first_report.shutdowns, 0);

    assert_eq!(
        update_until_state(&mut server, first.id(), AssetLoadState::UploadingGpu),
        AssetLoadState::UploadingGpu
    );
    finish_uploads(&mut server);
    assert_eq!(server.state(&first), AssetLoadState::Ready);

    let after_first_report = server.async_worker_pool_report();
    assert_eq!(after_first_report.active_workers, 1);
    assert_eq!(after_first_report.in_flight_jobs, 0);
    assert_eq!(after_first_report.completed_jobs, 1);
    assert_eq!(after_first_report.worker_threads_started, 1);

    let second: Handle<Texture> = server.load("textures/pool_b.texture");
    server.update_loading();
    let second_report = server.async_worker_pool_report();
    assert_eq!(second_report.active_workers, 1);
    assert_eq!(second_report.in_flight_jobs, 1);
    assert_eq!(second_report.dispatched_jobs, 2);
    assert_eq!(second_report.worker_threads_started, 1);

    assert_eq!(
        update_until_state(&mut server, second.id(), AssetLoadState::UploadingGpu),
        AssetLoadState::UploadingGpu
    );
    finish_uploads(&mut server);
    assert_eq!(server.state(&second), AssetLoadState::Ready);

    let shutdown_report = server.shutdown_async_worker_pool();
    assert_eq!(shutdown_report.desired_workers, 1);
    assert_eq!(shutdown_report.active_workers, 0);
    assert_eq!(shutdown_report.in_flight_jobs, 0);
    assert_eq!(shutdown_report.dispatched_jobs, 2);
    assert_eq!(shutdown_report.completed_jobs, 2);
    assert_eq!(shutdown_report.worker_threads_started, 1);
    assert_eq!(shutdown_report.shutdowns, 1);

    let second_shutdown_report = server.shutdown_async_worker_pool();
    assert_eq!(second_shutdown_report.active_workers, 0);
    assert_eq!(second_shutdown_report.shutdowns, 1);
}

#[cfg(feature = "async_loading")]
#[test]
fn async_loading_can_cancel_in_flight_request_before_result_applies() {
    let mut server = server_with_textures_and_config(
        &[("textures/cancel_async.texture", 12)],
        AssetServerConfig {
            enable_async_loading: true,
            max_io_jobs_per_frame: 1,
            max_cpu_jobs_per_frame: 1,
            ..AssetServerConfig::default()
        },
    );
    let texture: Handle<Texture> = server.load("textures/cancel_async.texture");

    server.update_loading();
    assert_eq!(server.state(&texture), AssetLoadState::LoadingBytes);
    let dispatched_report = server.async_worker_pool_report();
    assert_eq!(dispatched_report.in_flight_jobs, 1);
    assert_eq!(dispatched_report.dispatched_jobs, 1);
    assert!(server.cancel_load_by_id(texture.id()));
    assert_eq!(server.state(&texture), AssetLoadState::Cancelled);
    assert_eq!(server.async_worker_pool_report().in_flight_jobs, 1);

    for _ in 0..50 {
        server.update_loading();
        if server.async_worker_pool_report().in_flight_jobs == 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(server.state(&texture), AssetLoadState::Cancelled);
    let collected_report = server.async_worker_pool_report();
    assert_eq!(collected_report.in_flight_jobs, 0);
    assert_eq!(collected_report.completed_jobs, 1);
    assert!(server.drain_gpu_uploads().next().is_none());
}

#[cfg(all(feature = "async_loading", feature = "parallel"))]
#[test]
fn async_parallel_loading_dispatches_multiple_workers_in_one_update() {
    let mut server = server_with_textures_and_config(
        &[
            ("textures/parallel_a.texture", 13),
            ("textures/parallel_b.texture", 14),
        ],
        AssetServerConfig {
            enable_async_loading: true,
            worker_threads: 2,
            max_io_jobs_per_frame: 64,
            max_cpu_jobs_per_frame: 64,
            ..AssetServerConfig::default()
        },
    );
    let a: Handle<Texture> = server.load("textures/parallel_a.texture");
    let b: Handle<Texture> = server.load("textures/parallel_b.texture");

    let report = server.loading_policy_report();
    assert_eq!(report.effective_worker_threads, 2);

    server.update_loading();
    assert_eq!(server.state(&a), AssetLoadState::LoadingBytes);
    assert_eq!(server.state(&b), AssetLoadState::LoadingBytes);
    let worker_report = server.async_worker_pool_report();
    assert_eq!(worker_report.active_workers, 2);
    assert_eq!(worker_report.in_flight_jobs, 2);
    assert_eq!(worker_report.dispatched_jobs, 2);
    assert_eq!(worker_report.worker_threads_started, 2);

    assert_eq!(
        update_until_state(&mut server, a.id(), AssetLoadState::UploadingGpu),
        AssetLoadState::UploadingGpu
    );
    assert_eq!(
        update_until_state(&mut server, b.id(), AssetLoadState::UploadingGpu),
        AssetLoadState::UploadingGpu
    );
}
