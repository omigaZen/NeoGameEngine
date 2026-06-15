use engine_asset::prelude::*;

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
}

fn server_with_io(io: MemoryAssetIo) -> AssetServer {
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_builtin_loaders();
    server
}

fn finish_uploads(server: &mut AssetServer, handle_start: u64) {
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(uploads.into_iter().enumerate().map(|(index, upload)| {
        GpuUploadResult::ok(upload.id, GpuResourceHandle(handle_start + index as u64))
    }));
}

fn event_position(events: &[AssetEvent], predicate: impl Fn(&AssetEvent) -> bool) -> usize {
    events
        .iter()
        .position(predicate)
        .expect("expected event was not emitted")
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HotReloadCustomAsset {
    source: String,
}

impl Asset for HotReloadCustomAsset {
    const TYPE_NAME: &'static str = "HotReloadCustomAsset";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0bad_cafe);
}

struct HotReloadCustomLoader;

impl AssetLoader for HotReloadCustomLoader {
    fn name(&self) -> &'static str {
        "HotReloadCustomLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["hcustom"]
    }

    fn asset_type(&self) -> AssetTypeId {
        HotReloadCustomAsset::TYPE_ID
    }

    fn load(
        &self,
        _ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
            message: error.to_string(),
        })?;
        if source.trim() == "fail" {
            return Err(AssetError::Decode {
                message: "custom reload decode failure".to_owned(),
            }
            .into());
        }
        Ok(LoadedAsset::new(HotReloadCustomAsset {
            source: source.to_owned(),
        }))
    }
}

#[test]
fn hot_reload_watcher_detects_metadata_change_and_queues_reload() {
    let path = AssetPath::parse("textures/watched.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load(path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);
    assert_eq!(server.get(&texture).unwrap().width, 1);
    server.watch_hot_reload_path(path.clone()).unwrap();

    server.set_io(MemoryAssetIo::new().with_file(path.path(), texture_bytes(3, 1, 30)));
    let report = server.poll_hot_reload_watches().unwrap();
    assert_eq!(report.watched_paths, 1);
    assert_eq!(report.unchanged_paths, 0);
    assert!(report.errors.is_empty());
    assert_eq!(report.changed.len(), 1);
    assert_eq!(report.changed[0].id, Some(texture.id()));
    assert_eq!(report.changed[0].path, path);

    server.update_hot_reload();
    assert_eq!(server.state(&texture), AssetLoadState::Reloading);
    server.update_loading();
    finish_uploads(&mut server, 40);

    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!(server.get(&texture).unwrap().width, 3);
}

#[test]
fn hot_reload_watcher_debounces_duplicate_pending_path_changes() {
    let path = AssetPath::parse("textures/debounced.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load(path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);
    server.watch_hot_reload_path(path.clone()).unwrap();

    server.set_io(MemoryAssetIo::new().with_file(path.path(), texture_bytes(2, 1, 20)));
    server.queue_hot_reload_path(path.clone());
    let report = server.poll_hot_reload_watches().unwrap();
    assert!(report.changed.is_empty());
    assert_eq!(report.debounced_changes, 1);
    assert!(report.errors.is_empty());

    server.update_hot_reload();
    server.update_loading();
    finish_uploads(&mut server, 50);

    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!(server.get(&texture).unwrap().width, 2);
}

#[test]
fn hot_reload_watcher_reports_batch_metadata_errors() {
    let path = AssetPath::parse("textures/missing_after_watch.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);
    server.watch_hot_reload_path(path.clone()).unwrap();

    server.set_io(MemoryAssetIo::new());
    let report = server.poll_hot_reload_watches().unwrap();
    assert_eq!(report.watched_paths, 1);
    assert_eq!(report.changed.len(), 0);
    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].path, path);
    assert!(matches!(
        report.errors[0].error,
        AssetIoError::NotFound { .. }
    ));
    assert_eq!(server.last_hot_reload_poll_report(), &report);
}

#[test]
fn hot_reload_watcher_replaces_existing_watch_with_latest_backend_and_metadata() {
    let path = AssetPath::parse("textures/replaced.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);

    server.watch_hot_reload_path(path.clone()).unwrap();
    assert_eq!(
        server.hot_reload_watch(&path).unwrap().backend,
        HotReloadWatchBackend::PollingMetadata
    );
    assert_eq!(server.hot_reload_watches().count(), 1);

    server.set_io(MemoryAssetIo::new().with_file(path.path(), texture_bytes(2, 1, 20)));
    server
        .watch_hot_reload_path_with_backend(path.clone(), HotReloadWatchBackend::AsyncNotification)
        .unwrap();
    let watch = server.hot_reload_watch(&path).unwrap();
    assert_eq!(watch.backend, HotReloadWatchBackend::AsyncNotification);
    assert_eq!(watch.last_metadata.size, 16);
    assert_eq!(server.hot_reload_watches().count(), 1);
}

#[test]
fn hot_reload_async_watch_backend_queues_notified_paths_without_polling() {
    let path = AssetPath::parse("textures/async_watch.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load(path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);
    assert_eq!(server.get(&texture).unwrap().width, 1);

    server
        .watch_hot_reload_path_with_backend(path.clone(), HotReloadWatchBackend::AsyncNotification)
        .unwrap();
    let start_report = server.start_hot_reload_async_watch_backend().unwrap();
    assert!(start_report.is_running());
    assert_eq!(start_report.watched_paths, 1);
    assert_eq!(start_report.pending_notifications, 0);

    server.set_io(MemoryAssetIo::new().with_file(path.path(), texture_bytes(2, 1, 20)));
    let quiet_poll = server.poll_hot_reload_watches().unwrap();
    assert_eq!(quiet_poll.watched_paths, 1);
    assert_eq!(quiet_poll.unchanged_paths, 0);
    assert_eq!(quiet_poll.async_notifications, 0);
    assert!(quiet_poll.changed.is_empty());
    assert_eq!(server.state(&texture), AssetLoadState::Ready);

    assert!(server
        .notify_hot_reload_async_watch_change(path.clone())
        .unwrap());
    let pending_report = server.hot_reload_async_watch_report();
    assert_eq!(pending_report.pending_notifications, 1);
    assert_eq!(pending_report.received_notifications, 1);

    let policy = server.hot_reload_policy_report();
    assert_eq!(
        policy.watch_backend,
        HotReloadWatchBackend::AsyncNotification
    );
    assert_eq!(
        policy.async_watch.lifecycle,
        HotReloadAsyncWatchLifecycle::Running
    );
    assert_eq!(
        policy.watch_statuses[0].backend,
        HotReloadWatchBackend::AsyncNotification
    );
    assert!(!policy.watch_statuses[0].queued);

    let report = server.poll_hot_reload_watches().unwrap();
    assert_eq!(report.async_notifications, 1);
    assert_eq!(report.changed.len(), 1);
    assert_eq!(report.changed[0].id, Some(texture.id()));
    assert_eq!(report.changed[0].path, path);
    assert!(report.errors.is_empty());
    assert_eq!(
        server
            .hot_reload_async_watch_report()
            .delivered_notifications,
        1
    );

    server.update_hot_reload();
    assert_eq!(server.state(&texture), AssetLoadState::Reloading);
    server.update_loading();
    finish_uploads(&mut server, 30);
    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!(server.get(&texture).unwrap().width, 2);
}

#[test]
fn hot_reload_async_watch_backend_reports_errors_and_stop_drops_pending_notifications() {
    let path = AssetPath::parse("textures/async_error.texture");
    let missing = AssetPath::parse("textures/missing_async_registration.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);

    let registration_error = server
        .watch_hot_reload_path_with_backend(missing, HotReloadWatchBackend::AsyncNotification)
        .unwrap_err();
    assert!(matches!(registration_error, AssetError::Io { .. }));

    server
        .watch_hot_reload_path_with_backend(path.clone(), HotReloadWatchBackend::AsyncNotification)
        .unwrap();
    server.start_hot_reload_async_watch_backend().unwrap();

    server.set_io(MemoryAssetIo::new());
    assert!(server
        .notify_hot_reload_async_watch_change(path.clone())
        .unwrap());
    let poll = server.poll_hot_reload_watches().unwrap();
    assert_eq!(poll.async_notifications, 1);
    assert!(poll.changed.is_empty());
    assert_eq!(poll.errors.len(), 1);
    assert_eq!(poll.errors[0].path, path);
    assert!(matches!(
        poll.errors[0].error,
        AssetIoError::NotFound { .. }
    ));

    let policy = server.hot_reload_policy_report();
    assert_eq!(policy.async_watch.errors, poll.errors);
    assert!(matches!(
        policy.watch_statuses[0].last_error,
        Some(AssetIoError::NotFound { .. })
    ));

    server.set_io(MemoryAssetIo::new().with_file(path.path(), texture_bytes(3, 1, 30)));
    assert!(server
        .notify_hot_reload_async_watch_change(path.clone())
        .unwrap());
    let before_stop = server.hot_reload_async_watch_report();
    assert_eq!(before_stop.pending_notifications, 1);
    let stop_report = server.stop_hot_reload_async_watch_backend().unwrap();
    assert_eq!(stop_report.lifecycle, HotReloadAsyncWatchLifecycle::Stopped);
    assert_eq!(stop_report.pending_notifications, 0);
    assert_eq!(stop_report.dropped_notifications, 1);

    assert!(!server
        .notify_hot_reload_async_watch_change(path.clone())
        .unwrap());
    assert_eq!(
        server.hot_reload_async_watch_report().dropped_notifications,
        2
    );
}

#[test]
fn hot_reload_async_watch_notification_is_dropped_after_unwatch() {
    let path = AssetPath::parse("textures/async_unwatch.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);

    server
        .watch_hot_reload_path_with_backend(path.clone(), HotReloadWatchBackend::AsyncNotification)
        .unwrap();
    server.start_hot_reload_async_watch_backend().unwrap();
    assert!(server
        .notify_hot_reload_async_watch_change(path.clone())
        .unwrap());
    assert!(server.unwatch_hot_reload_path(&path));

    let report = server.poll_hot_reload_watches().unwrap();
    assert_eq!(report.async_notifications, 1);
    assert_eq!(report.dropped_notifications, 1);
    assert!(report.changed.is_empty());
    assert!(report.errors.is_empty());
    assert!(server.hot_reload_watch(&path).is_none());
}

#[test]
fn hot_reload_async_watch_backend_start_and_stop_are_idempotent() {
    let path = AssetPath::parse("textures/async_idempotent.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);

    server
        .watch_hot_reload_path_with_backend(path.clone(), HotReloadWatchBackend::AsyncNotification)
        .unwrap();

    let first_start = server.start_hot_reload_async_watch_backend().unwrap();
    assert!(first_start.is_running());
    assert_eq!(first_start.watched_paths, 1);
    assert_eq!(first_start.pending_notifications, 0);

    let second_start = server.start_hot_reload_async_watch_backend().unwrap();
    assert!(second_start.is_running());
    assert_eq!(second_start.watched_paths, 1);
    assert_eq!(second_start.pending_notifications, 0);

    assert!(server
        .notify_hot_reload_async_watch_change(path.clone())
        .unwrap());
    assert_eq!(
        server.hot_reload_async_watch_report().pending_notifications,
        1
    );

    let first_stop = server.stop_hot_reload_async_watch_backend().unwrap();
    assert!(!first_stop.is_running());
    assert_eq!(first_stop.pending_notifications, 0);
    assert_eq!(first_stop.dropped_notifications, 1);

    let second_stop = server.stop_hot_reload_async_watch_backend().unwrap();
    assert!(!second_stop.is_running());
    assert_eq!(second_stop.pending_notifications, 0);
    assert_eq!(second_stop.dropped_notifications, 1);

    assert!(!server.notify_hot_reload_async_watch_change(path).unwrap());
    assert_eq!(
        server.hot_reload_async_watch_report().dropped_notifications,
        2
    );
}

#[test]
fn hot_reload_success_keeps_handle_id_and_replaces_asset_after_upload() {
    let path = AssetPath::parse("textures/hero.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load(path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);
    assert_eq!(server.get(&texture).unwrap().width, 1);

    let replacement = MemoryAssetIo::new().with_file(path.path(), texture_bytes(2, 1, 20));
    server.set_io(replacement);
    let event_start = server.events().len();
    server.queue_hot_reload_id(texture.id()).unwrap();
    server.update_hot_reload();
    assert_eq!(server.state(&texture), AssetLoadState::Reloading);
    assert!(server.get(&texture).is_none());
    assert_eq!(
        server
            .storage::<Texture>()
            .unwrap()
            .get_cpu_by_id(texture.id())
            .unwrap()
            .width,
        1
    );
    server.update_loading();
    assert_eq!(server.state(&texture), AssetLoadState::UploadingGpu);
    assert_eq!(
        server
            .storage::<Texture>()
            .unwrap()
            .get_cpu_by_id(texture.id())
            .unwrap()
            .width,
        1
    );
    finish_uploads(&mut server, 10);

    let reloaded = server.get(&texture).unwrap();
    assert_eq!(texture.id(), server.id_from_path(&path).unwrap());
    assert_eq!((reloaded.width, reloaded.height), (2, 1));
    assert_eq!(reloaded.gpu, Some(GpuResourceHandle(10)));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Reloaded { id } if *id == texture.id())));
    let reload_events = &server.events()[event_start..];
    let reload_started = event_position(
        reload_events,
        |event| matches!(event, AssetEvent::ReloadStarted { id } if *id == texture.id()),
    );
    let load_requested = event_position(
        reload_events,
        |event| matches!(event, AssetEvent::LoadRequested { id, .. } if *id == texture.id()),
    );
    let upload_queued = event_position(
        reload_events,
        |event| matches!(event, AssetEvent::GpuUploadQueued { id } if *id == texture.id()),
    );
    let reloaded_event = event_position(
        reload_events,
        |event| matches!(event, AssetEvent::Reloaded { id } if *id == texture.id()),
    );
    let upload_finished = event_position(
        reload_events,
        |event| matches!(event, AssetEvent::GpuUploadFinished { id } if *id == texture.id()),
    );
    let ready = event_position(
        reload_events,
        |event| matches!(event, AssetEvent::Ready { id } if *id == texture.id()),
    );
    assert!(reload_started < load_requested);
    assert!(load_requested < upload_queued);
    assert!(upload_queued < reloaded_event);
    assert!(reloaded_event < upload_finished);
    assert!(upload_finished < ready);
}

#[test]
fn hot_reload_decode_failure_rolls_back_to_previous_ready_asset() {
    let path = AssetPath::parse("textures/hero.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load(path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);

    server.set_io(MemoryAssetIo::new().with_file(path.path(), vec![1, 2, 3]));
    server.queue_hot_reload_path(path);
    server.update_hot_reload();
    assert_eq!(server.state(&texture), AssetLoadState::Reloading);
    server.update_loading();

    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!(server.get(&texture).unwrap().width, 1);
    assert!(matches!(
        server.error_by_id(texture.id()),
        Some(AssetError::Decode { .. })
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == texture.id())));
}

#[test]
fn reload_failure_diagnostic_is_cleared_by_later_successful_reload() {
    let path = AssetPath::parse("textures/hero.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load(path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);

    server.set_io(MemoryAssetIo::new().with_file(path.path(), vec![1, 2, 3]));
    server.reload_by_id(texture.id()).unwrap();
    server.update_loading();
    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!(server.get(&texture).unwrap().width, 1);
    assert_eq!(
        server.metadata(texture.id()).unwrap().path,
        Some(path.clone())
    );
    assert!(matches!(
        server.error_by_id(texture.id()),
        Some(AssetError::Decode { .. })
    ));

    server.set_io(MemoryAssetIo::new().with_file(path.path(), texture_bytes(2, 1, 20)));
    server.reload_by_id(texture.id()).unwrap();
    server.update_loading();
    finish_uploads(&mut server, 20);

    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!(server.get(&texture).unwrap().width, 2);
    assert_eq!(server.metadata(texture.id()).unwrap().path, Some(path));
    assert!(server.error_by_id(texture.id()).is_none());
}

#[test]
fn reload_by_id_reports_missing_asset_without_mutating_state() {
    let path = AssetPath::parse("textures/reload_by_id.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load(path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);
    assert_eq!(server.get(&texture).unwrap().width, 1);

    let missing = AssetId::new();
    let error = server.reload_by_id(missing).unwrap_err();
    assert!(matches!(error, AssetError::AssetNotFound { id } if id == missing));
    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!(server.get(&texture).unwrap().width, 1);
    assert_eq!(server.metadata(texture.id()).unwrap().path, Some(path));
    assert!(server.error_by_id(texture.id()).is_none());
}

#[test]
fn reload_by_path_clears_failure_diagnostic_by_later_successful_reload() {
    let path = AssetPath::parse("textures/by_path.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load(path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);

    server.set_io(MemoryAssetIo::new().with_file(path.path(), vec![1, 2, 3]));
    server.reload_by_path(&path).unwrap();
    server.update_loading();
    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!(server.get(&texture).unwrap().width, 1);
    assert_eq!(
        server.metadata(texture.id()).unwrap().path,
        Some(path.clone())
    );
    assert!(matches!(
        server.error_by_id(texture.id()),
        Some(AssetError::Decode { .. })
    ));

    server.set_io(MemoryAssetIo::new().with_file(path.path(), texture_bytes(3, 1, 30)));
    server.reload_by_path(&path).unwrap();
    server.update_loading();
    finish_uploads(&mut server, 30);

    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!(server.get(&texture).unwrap().width, 3);
    assert_eq!(server.metadata(texture.id()).unwrap().path, Some(path));
    assert!(server.error_by_id(texture.id()).is_none());
}

#[test]
fn reload_by_path_reports_missing_path_without_mutating_state() {
    let path = AssetPath::parse("textures/missing.texture");
    let loaded_path = AssetPath::parse("textures/loaded.texture");
    let io = MemoryAssetIo::new().with_file(loaded_path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load(loaded_path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);
    assert_eq!(server.get(&texture).unwrap().width, 1);

    let error = server.reload_by_path(&path).unwrap_err();
    assert!(matches!(
        error,
        AssetError::PathNotFound { path: missing } if missing == path
    ));
    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!(server.get(&texture).unwrap().width, 1);
    assert_eq!(
        server.metadata(texture.id()).unwrap().path,
        Some(loaded_path)
    );
    assert!(server.error_by_id(texture.id()).is_none());
}

#[test]
fn queue_hot_reload_id_reports_missing_asset_without_mutating_state() {
    let path = AssetPath::parse("textures/queued.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load(path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);
    assert_eq!(server.get(&texture).unwrap().width, 1);

    let missing = AssetId::new();
    let error = server.queue_hot_reload_id(missing).unwrap_err();
    assert!(matches!(error, AssetError::AssetNotFound { id } if id == missing));
    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!(server.get(&texture).unwrap().width, 1);
    assert_eq!(server.metadata(texture.id()).unwrap().path, Some(path));
    assert!(server.error_by_id(texture.id()).is_none());
}

#[test]
fn hot_reload_gpu_failure_rolls_back_without_replacing_old_asset() {
    let path = AssetPath::parse("textures/hero.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load(path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);

    server.set_io(MemoryAssetIo::new().with_file(path.path(), texture_bytes(3, 1, 30)));
    server.queue_hot_reload_id(texture.id()).unwrap();
    server.update_hot_reload();
    server.update_loading();
    assert_eq!(server.state(&texture), AssetLoadState::UploadingGpu);
    assert_eq!(
        server
            .storage::<Texture>()
            .unwrap()
            .get_cpu_by_id(texture.id())
            .unwrap()
            .width,
        1
    );
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::failed(upload.id, "simulated upload failure")),
    );

    let kept = server.get(&texture).unwrap();
    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    assert_eq!((kept.width, kept.height), (1, 1));
    assert_eq!(kept.gpu, Some(GpuResourceHandle(1)));
    assert!(matches!(
        server.error_by_id(texture.id()),
        Some(AssetError::GpuUpload { .. })
    ));
}

#[test]
fn hot_reload_rollback_policy_reports_builtin_and_custom_retention() {
    let custom_type = HotReloadCustomAsset::TYPE_ID;
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_asset_type::<HotReloadCustomAsset>();

    let texture_policy = server.hot_reload_rollback_policy_for_type(AssetTypeId::of::<Texture>());
    assert_eq!(texture_policy.type_name, "Texture");
    assert_eq!(
        texture_policy.retention,
        HotReloadRollbackRetention::CpuAndGpu
    );
    assert!(texture_policy.can_retain_previous_ready_state());
    assert!(texture_policy.retention.retains_cpu());
    assert!(texture_policy.retention.retains_gpu());

    let custom_policy = server.hot_reload_rollback_policy_for_type(custom_type);
    assert_eq!(custom_policy.type_name, "HotReloadCustomAsset");
    assert_eq!(custom_policy.retention, HotReloadRollbackRetention::Cpu);
    assert!(custom_policy.can_retain_previous_ready_state());
    assert!(custom_policy.retention.retains_cpu());
    assert!(!custom_policy.retention.retains_gpu());

    let missing_policy = server.hot_reload_rollback_policy_for_type(AssetTypeId::from_u128(0xabc));
    assert_eq!(missing_policy.retention, HotReloadRollbackRetention::None);
    assert!(!missing_policy.can_retain_previous_ready_state());

    let policies = server.hot_reload_rollback_policies();
    assert!(policies
        .iter()
        .any(|policy| policy.asset_type == custom_type
            && policy.retention == HotReloadRollbackRetention::Cpu));
}

#[test]
fn custom_asset_reload_failure_reports_and_rolls_back_cpu_state() {
    let path = AssetPath::parse("custom/hero.hcustom");
    let io = MemoryAssetIo::new().with_file(path.path(), "version=1");
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_loader(HotReloadCustomLoader);

    let custom: Handle<HotReloadCustomAsset> = server.load(path.clone());
    server.update_loading();
    assert_eq!(server.state(&custom), AssetLoadState::Ready);
    assert_eq!(server.get(&custom).unwrap().source, "version=1");

    let report = server
        .hot_reload_rollback_report_by_id(custom.id())
        .unwrap();
    assert_eq!(report.id, custom.id());
    assert_eq!(report.path, Some(path.clone()));
    assert_eq!(report.current_state, AssetLoadState::Ready);
    assert!(report.can_rollback_now);
    assert_eq!(report.policy.type_name, "HotReloadCustomAsset");
    assert_eq!(report.policy.retention, HotReloadRollbackRetention::Cpu);

    server.set_io(MemoryAssetIo::new().with_file(path.path(), "fail"));
    server.reload_by_id(custom.id()).unwrap();
    server.update_loading();

    assert_eq!(server.state(&custom), AssetLoadState::Ready);
    assert_eq!(server.get(&custom).unwrap().source, "version=1");
    assert!(matches!(
        server.error_by_id(custom.id()),
        Some(AssetError::Decode { message }) if message.contains("custom reload decode failure")
    ));
}

#[test]
fn hot_reload_rollback_override_can_disable_custom_asset_retention() {
    let path = AssetPath::parse("custom/no_rollback.hcustom");
    let io = MemoryAssetIo::new().with_file(path.path(), "version=1");
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_loader(HotReloadCustomLoader);
    let custom: Handle<HotReloadCustomAsset> = server.load(path.clone());
    server.update_loading();
    assert_eq!(server.state(&custom), AssetLoadState::Ready);

    server.set_hot_reload_rollback_override(
        HotReloadCustomAsset::TYPE_ID,
        HotReloadRollbackRetention::None,
    );
    let report = server
        .hot_reload_rollback_report_by_id(custom.id())
        .unwrap();
    assert_eq!(report.policy.retention, HotReloadRollbackRetention::None);
    assert!(report.policy.overridden);
    assert!(!report.can_rollback_now);

    server.set_io(MemoryAssetIo::new().with_file(path.path(), "fail"));
    server.reload_by_id(custom.id()).unwrap();
    server.update_loading();

    assert_eq!(server.state(&custom), AssetLoadState::Failed);
    assert!(server.get(&custom).is_none());
    assert!(matches!(
        server.error_by_id(custom.id()),
        Some(AssetError::Decode { message }) if message.contains("custom reload decode failure")
    ));

    assert_eq!(
        server.clear_hot_reload_rollback_override(HotReloadCustomAsset::TYPE_ID),
        Some(HotReloadRollbackRetention::None)
    );
    assert!(
        !server
            .hot_reload_rollback_policy_for_type(HotReloadCustomAsset::TYPE_ID)
            .overridden
    );
}

#[test]
fn hot_reload_policy_report_combines_config_watches_queue_and_rollback_policies() {
    let path = AssetPath::parse("textures/report.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 7));
    let mut config = AssetServerConfig::default();
    config.hot_reload_dependency_policy = HotReloadDependencyPolicy::Transitive;
    let mut server = AssetServer::new(config);
    server.set_io(io);
    server.register_builtin_loaders();
    let texture: Handle<Texture> = server.load(path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);
    server.watch_hot_reload_path(path.clone()).unwrap();
    server.queue_hot_reload_id(texture.id()).unwrap();

    let event_count = server.events().len();
    let report = server.hot_reload_policy_report();
    assert_eq!(
        report.dependency_policy,
        HotReloadDependencyPolicy::Transitive
    );
    assert_eq!(report.watched_paths(), 1);
    assert_eq!(report.watch_backend, HotReloadWatchBackend::PollingMetadata);
    assert_eq!(report.watches[0].path, path);
    assert_eq!(report.watch_statuses.len(), 1);
    assert_eq!(report.watch_statuses[0].path, path);
    assert_eq!(
        report.watch_statuses[0].backend,
        HotReloadWatchBackend::PollingMetadata
    );
    assert!(report.watch_statuses[0].queued);
    assert!(report.watch_statuses[0].last_error.is_none());
    assert_eq!(report.queued_changes(), 1);
    assert_eq!(report.queued_changes[0].id, Some(texture.id()));
    assert_eq!(report.last_poll, HotReloadPollReport::default());
    assert!(report.rollback_policies.iter().any(|policy| {
        policy.asset_type == AssetTypeId::of::<Texture>()
            && policy.retention == HotReloadRollbackRetention::CpuAndGpu
    }));
    assert_eq!(server.events().len(), event_count);
    assert_eq!(server.state(&texture), AssetLoadState::Ready);

    server.set_io(MemoryAssetIo::new());
    let poll = server.poll_hot_reload_watches().unwrap();
    assert_eq!(poll.errors.len(), 1);
    let report = server.hot_reload_policy_report();
    assert_eq!(report.last_poll.errors.len(), 1);
    assert!(matches!(
        report.watch_statuses[0].last_error,
        Some(AssetIoError::NotFound { .. })
    ));
    assert!(report.watch_statuses[0].queued);
}

#[test]
fn hot_reload_dependency_change_reloads_dependent_material() {
    let shader_path = AssetPath::parse("shaders/pbr.wgsl");
    let material_path = AssetPath::parse("materials/hero.material");
    let mut io = MemoryAssetIo::new();
    io.insert(shader_path.path(), "@fragment fn main() {}");
    io.insert(
        material_path.path(),
        "name=hero\nshader=shaders/pbr.wgsl\nbase_color=1,1,1,1\n",
    );
    let mut server = server_with_io(io);
    let material: Handle<Material> = server.load(material_path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);
    finish_uploads(&mut server, 2);
    assert!(server.is_ready(&material));
    let shader_id = server.id_from_path(&shader_path).unwrap();
    let material_id = material.id();
    assert!(server
        .dependency_graph()
        .reverse_dependencies(shader_id)
        .contains(&material_id));

    let mut replacement = MemoryAssetIo::new();
    replacement.insert(shader_path.path(), "@fragment fn main() { }");
    replacement.insert(
        material_path.path(),
        "name=hero_reloaded\nshader=shaders/pbr.wgsl\nbase_color=1,1,1,1\n",
    );
    server.set_io(replacement);
    server.queue_hot_reload_id(shader_id).unwrap();
    server.update_hot_reload();

    server.update_loading();
    finish_uploads(&mut server, 20);
    finish_uploads(&mut server, 30);

    assert!(server.is_ready(&material));
    assert_eq!(
        server.get(&material).unwrap().name.as_deref(),
        Some("hero_reloaded")
    );
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Reloaded { id } if *id == shader_id)));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Reloaded { id } if *id == material_id)));
}

#[test]
fn hot_reload_dependency_plan_reports_dependents_without_mutating_state() {
    let shader_path = AssetPath::parse("shaders/pbr.wgsl");
    let material_path = AssetPath::parse("materials/hero.material");
    let scene_path = AssetPath::parse("scenes/hero.scene");
    let mut io = MemoryAssetIo::new();
    io.insert(shader_path.path(), "@fragment fn main() {}");
    io.insert(
        material_path.path(),
        "name=hero\nshader=shaders/pbr.wgsl\nbase_color=1,1,1,1\n",
    );
    io.insert(
        scene_path.path(),
        "NGA_SCENE_V1\nname=hero_scene\ndependency=materials/hero.material\nentity=Hero\ncomponent=Transform|position=0,0,0\n",
    );
    let mut server = server_with_io(io);
    let scene: Handle<SceneAsset> = server.load(scene_path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);
    finish_uploads(&mut server, 10);
    assert!(server.is_ready(&scene));

    let shader_id = server.id_from_path(&shader_path).unwrap();
    let material_id = server.id_from_path(&material_path).unwrap();
    let scene_id = scene.id();
    let event_count = server.events().len();
    let shader_state = server.state_by_id(shader_id);
    let material_state = server.state_by_id(material_id);
    let scene_state = server.state_by_id(scene_id);

    let direct = server
        .hot_reload_dependency_plan_by_id(shader_id, HotReloadDependencyPolicy::Direct)
        .unwrap();
    assert_eq!(direct.changed, shader_id);
    assert_eq!(direct.changed_path, Some(shader_path.clone()));
    assert_eq!(direct.policy, HotReloadDependencyPolicy::Direct);
    assert_eq!(direct.dependents, vec![material_id]);
    assert!(direct.has_dependents());
    assert_eq!(direct.reload_order(), vec![shader_id, material_id]);
    assert_eq!(
        server
            .hot_reload_dependency_plan_by_path(&shader_path, HotReloadDependencyPolicy::Direct)
            .unwrap(),
        direct
    );

    let transitive = server
        .hot_reload_dependency_plan_by_id(shader_id, HotReloadDependencyPolicy::Transitive)
        .unwrap();
    assert_eq!(transitive.policy, HotReloadDependencyPolicy::Transitive);
    assert_eq!(transitive.dependents, vec![material_id, scene_id]);
    assert_eq!(
        transitive.reload_order(),
        vec![shader_id, material_id, scene_id]
    );

    assert_eq!(server.events().len(), event_count);
    assert_eq!(server.state_by_id(shader_id), shader_state);
    assert_eq!(server.state_by_id(material_id), material_state);
    assert_eq!(server.state_by_id(scene_id), scene_state);
}

#[test]
fn hot_reload_transitive_dependency_policy_reloads_nested_dependents() {
    let shader_path = AssetPath::parse("shaders/pbr.wgsl");
    let material_path = AssetPath::parse("materials/hero.material");
    let scene_path = AssetPath::parse("scenes/hero.scene");
    let mut io = MemoryAssetIo::new();
    io.insert(shader_path.path(), "@fragment fn main() {}");
    io.insert(
        material_path.path(),
        "name=hero\nshader=shaders/pbr.wgsl\nbase_color=1,1,1,1\n",
    );
    io.insert(
        scene_path.path(),
        "NGA_SCENE_V1\nname=hero_scene\ndependency=materials/hero.material\nentity=Hero\ncomponent=Transform|position=0,0,0\n",
    );
    let mut config = AssetServerConfig::default();
    config.hot_reload_dependency_policy = HotReloadDependencyPolicy::Transitive;
    let mut server = AssetServer::new(config);
    server.set_io(io);
    server.register_builtin_loaders();
    let scene: Handle<SceneAsset> = server.load(scene_path.clone());
    server.update_loading();
    finish_uploads(&mut server, 1);
    finish_uploads(&mut server, 10);
    assert!(server.is_ready(&scene));

    let shader_id = server.id_from_path(&shader_path).unwrap();
    let material_id = server.id_from_path(&material_path).unwrap();
    let scene_id = scene.id();
    let mut replacement = MemoryAssetIo::new();
    replacement.insert(shader_path.path(), "@fragment fn main() { }");
    replacement.insert(
        material_path.path(),
        "name=hero_reloaded\nshader=shaders/pbr.wgsl\nbase_color=1,1,1,1\n",
    );
    replacement.insert(
        scene_path.path(),
        "NGA_SCENE_V1\nname=hero_scene_reloaded\ndependency=materials/hero.material\nentity=Hero\ncomponent=Transform|position=1,0,0\n",
    );
    server.set_io(replacement);
    server.queue_hot_reload_id(shader_id).unwrap();
    server.update_hot_reload();

    assert_eq!(server.state_by_id(shader_id), AssetLoadState::Reloading);
    assert_eq!(server.state_by_id(material_id), AssetLoadState::Reloading);
    assert_eq!(server.state_by_id(scene_id), AssetLoadState::Reloading);
    server.update_loading();
    finish_uploads(&mut server, 20);
    finish_uploads(&mut server, 30);

    assert_eq!(
        server.get(&scene).unwrap().name.as_str(),
        "hero_scene_reloaded"
    );
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Reloaded { id } if *id == shader_id)));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Reloaded { id } if *id == material_id)));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Reloaded { id } if *id == scene_id)));
}

#[test]
fn dependency_graph_transitive_dependents_are_deterministic_and_cycle_safe() {
    let root = AssetId::from_u128(10);
    let direct_high = AssetId::from_u128(30);
    let direct_low = AssetId::from_u128(20);
    let child = AssetId::from_u128(40);
    let mut graph = DependencyGraph::new();

    graph.set_dependencies(direct_high, vec![root]);
    graph.set_dependencies(direct_low, vec![root]);
    graph.set_dependencies(child, vec![direct_high, direct_low]);
    graph.set_dependencies(root, vec![child]);

    assert_eq!(graph.direct_dependents(root), vec![direct_low, direct_high]);
    assert_eq!(
        graph.transitive_dependents(root),
        vec![direct_low, direct_high, child]
    );
}
