use std::{collections::HashMap, sync::Arc, time::Instant};

use engine_platform::{
    FileSystem, FullscreenMode, LogicalSize, PhysicalSize, PlatformContext, PlatformError,
    PlatformEvent, PlatformResult, PlatformWindow, RunMode, WindowDesc, WindowEvent, WindowId,
};
use winit::{
    dpi::LogicalSize as WinitLogicalSize,
    event_loop::ActiveEventLoop,
    window::{Fullscreen, Window as WinitRawWindow, WindowId as WinitRawWindowId},
};

use crate::{fs::NativeFileSystem, window::WinitWindow};

pub(crate) struct WinitState {
    windows: HashMap<WindowId, WinitWindow>,
    winit_to_engine: HashMap<WinitRawWindowId, WindowId>,
    next_window_id: u64,
    primary_window: Option<WindowId>,
    should_exit: bool,
    run_mode: RunMode,
    fs: NativeFileSystem,
    current_modifiers: engine_platform::Modifiers,
    pending_events: Vec<PlatformEvent>,
}

impl WinitState {
    pub(crate) fn new() -> Self {
        Self {
            windows: HashMap::new(),
            winit_to_engine: HashMap::new(),
            next_window_id: 1,
            primary_window: None,
            should_exit: false,
            run_mode: RunMode::Wait,
            fs: NativeFileSystem::new(),
            current_modifiers: engine_platform::Modifiers::default(),
            pending_events: Vec::new(),
        }
    }

    pub(crate) fn window_id_for(&self, winit_id: WinitRawWindowId) -> Option<WindowId> {
        self.winit_to_engine.get(&winit_id).copied()
    }

    pub(crate) fn current_modifiers(&self) -> engine_platform::Modifiers {
        self.current_modifiers
    }

    pub(crate) fn set_current_modifiers(&mut self, modifiers: engine_platform::Modifiers) {
        self.current_modifiers = modifiers;
    }

    pub(crate) fn run_mode(&self) -> RunMode {
        self.run_mode
    }

    pub(crate) fn should_exit(&self) -> bool {
        self.should_exit
    }

    pub(crate) fn request_redraw_all(&self) {
        for window in self.windows.values() {
            window.request_redraw();
        }
    }

    pub(crate) fn window_scale_factor(&self, id: WindowId) -> f64 {
        self.windows
            .get(&id)
            .map(WinitWindow::scale_factor)
            .unwrap_or(1.0)
    }

    pub(crate) fn window_inner_size(&self, id: WindowId) -> Option<PhysicalSize<u32>> {
        self.windows.get(&id).map(PlatformWindow::inner_size)
    }

    pub(crate) fn take_pending_events(&mut self) -> Vec<PlatformEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn next_window_id(&mut self) -> PlatformResult<WindowId> {
        let id = WindowId(self.next_window_id);
        self.next_window_id = self
            .next_window_id
            .checked_add(1)
            .ok_or_else(|| PlatformError::BackendError("window id overflow".to_owned()))?;
        Ok(id)
    }

    fn refresh_primary_after_removal(&mut self, removed: WindowId) {
        if self.primary_window == Some(removed) {
            self.primary_window = self.windows.keys().next().copied();
        }
    }
}

pub(crate) struct WinitFrameContext<'a> {
    event_loop: &'a ActiveEventLoop,
    state: &'a mut WinitState,
}

impl<'a> WinitFrameContext<'a> {
    pub(crate) fn new(event_loop: &'a ActiveEventLoop, state: &'a mut WinitState) -> Self {
        Self { event_loop, state }
    }
}

impl PlatformContext for WinitFrameContext<'_> {
    fn create_window(&mut self, desc: WindowDesc) -> PlatformResult<WindowId> {
        let WindowDesc {
            title,
            size: LogicalSize { width, height },
            min_size,
            max_size,
            resizable,
            decorations,
            transparent,
            visible,
            fullscreen,
        } = desc;

        let mut attributes = WinitRawWindow::default_attributes()
            .with_title(title.clone())
            .with_inner_size(WinitLogicalSize::new(width, height))
            .with_resizable(resizable)
            .with_decorations(decorations)
            .with_transparent(transparent)
            .with_visible(visible);

        if let Some(size) = min_size {
            attributes =
                attributes.with_min_inner_size(WinitLogicalSize::new(size.width, size.height));
        }

        if let Some(size) = max_size {
            attributes =
                attributes.with_max_inner_size(WinitLogicalSize::new(size.width, size.height));
        }

        if fullscreen == FullscreenMode::Borderless {
            attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        let raw_window = self
            .event_loop
            .create_window(attributes)
            .map_err(|err| PlatformError::WindowCreationFailed(err.to_string()))?;
        let winit_id = raw_window.id();
        let id = self.state.next_window_id()?;
        let window = WinitWindow::new(id, Arc::new(raw_window), title);

        self.state.windows.insert(id, window);
        self.state.winit_to_engine.insert(winit_id, id);
        self.state.primary_window.get_or_insert(id);
        self.state.pending_events.push(PlatformEvent::Window {
            id,
            event: WindowEvent::Created,
        });

        Ok(id)
    }

    fn destroy_window(&mut self, id: WindowId) -> PlatformResult<()> {
        let window = self
            .state
            .windows
            .remove(&id)
            .ok_or(PlatformError::WindowNotFound)?;
        self.state.winit_to_engine.remove(&window.winit_id());
        self.state.refresh_primary_after_removal(id);
        self.state.pending_events.push(PlatformEvent::Window {
            id,
            event: WindowEvent::Destroyed,
        });
        Ok(())
    }

    fn window(&self, id: WindowId) -> Option<&dyn PlatformWindow> {
        self.state
            .windows
            .get(&id)
            .map(|window| window as &dyn PlatformWindow)
    }

    fn primary_window(&self) -> Option<WindowId> {
        self.state.primary_window
    }

    fn request_redraw(&mut self, id: WindowId) {
        if let Some(window) = self.state.windows.get(&id) {
            window.request_redraw();
        }
    }

    fn request_redraw_all(&mut self) {
        self.state.request_redraw_all();
    }

    fn set_run_mode(&mut self, mode: RunMode) {
        self.state.run_mode = mode;
    }

    fn exit(&mut self) {
        self.state.should_exit = true;
        self.event_loop.exit();
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn file_system(&self) -> &dyn FileSystem {
        &self.state.fs
    }
}
