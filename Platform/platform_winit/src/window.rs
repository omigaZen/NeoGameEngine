use std::sync::{Arc, RwLock};

use engine_platform::{
    CursorGrabMode, CursorIcon, LogicalPosition, PhysicalSize, PlatformError, PlatformResult,
    PlatformWindow, WindowId,
};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use winit::{
    dpi::LogicalPosition as WinitLogicalPosition,
    window::{
        CursorGrabMode as WinitCursorGrabMode, CursorIcon as WinitCursorIcon,
        Window as WinitRawWindow, WindowId as WinitRawWindowId,
    },
};

#[derive(Debug)]
pub struct WinitWindow {
    id: WindowId,
    inner: Arc<WinitRawWindow>,
    title: RwLock<String>,
}

impl WinitWindow {
    pub(crate) fn new(id: WindowId, inner: Arc<WinitRawWindow>, title: String) -> Self {
        Self {
            id,
            inner,
            title: RwLock::new(title),
        }
    }

    pub(crate) fn winit_id(&self) -> WinitRawWindowId {
        self.inner.id()
    }

    pub(crate) fn scale_factor(&self) -> f64 {
        self.inner.scale_factor()
    }

    fn cached_title(&self) -> String {
        match self.title.read() {
            Ok(title) => title.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    fn set_cached_title(&self, title: &str) {
        match self.title.write() {
            Ok(mut cached) => *cached = title.to_owned(),
            Err(poisoned) => *poisoned.into_inner() = title.to_owned(),
        }
    }
}

impl HasWindowHandle for WinitWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.inner.window_handle()
    }
}

impl HasDisplayHandle for WinitWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.inner.display_handle()
    }
}

impl PlatformWindow for WinitWindow {
    fn id(&self) -> WindowId {
        self.id
    }

    fn title(&self) -> String {
        let current = self.inner.title();
        if current.is_empty() {
            self.cached_title()
        } else {
            current
        }
    }

    fn set_title(&self, title: &str) {
        self.inner.set_title(title);
        self.set_cached_title(title);
    }

    fn inner_size(&self) -> PhysicalSize<u32> {
        let size = self.inner.inner_size();
        PhysicalSize {
            width: size.width,
            height: size.height,
        }
    }

    fn outer_size(&self) -> PhysicalSize<u32> {
        let size = self.inner.outer_size();
        PhysicalSize {
            width: size.width,
            height: size.height,
        }
    }

    fn scale_factor(&self) -> f64 {
        self.inner.scale_factor()
    }

    fn is_focused(&self) -> bool {
        self.inner.has_focus()
    }

    fn is_visible(&self) -> bool {
        self.inner.is_visible().unwrap_or(true)
    }

    fn set_visible(&self, visible: bool) {
        self.inner.set_visible(visible);
    }

    fn set_resizable(&self, resizable: bool) {
        self.inner.set_resizable(resizable);
    }

    fn set_cursor_visible(&self, visible: bool) {
        self.inner.set_cursor_visible(visible);
    }

    fn set_cursor_icon(&self, icon: CursorIcon) {
        self.inner.set_cursor(convert_cursor_icon(icon));
    }

    fn set_cursor_grab(&self, mode: CursorGrabMode) -> PlatformResult<()> {
        self.inner
            .set_cursor_grab(convert_cursor_grab_mode(mode))
            .map_err(|err| PlatformError::CursorOperationFailed(err.to_string()))
    }

    fn set_cursor_position(&self, position: LogicalPosition<f64>) -> PlatformResult<()> {
        self.inner
            .set_cursor_position(WinitLogicalPosition::new(position.x, position.y))
            .map_err(|err| PlatformError::CursorOperationFailed(err.to_string()))
    }

    fn request_redraw(&self) {
        self.inner.request_redraw();
    }
}

fn convert_cursor_icon(icon: CursorIcon) -> WinitCursorIcon {
    match icon {
        CursorIcon::Default => WinitCursorIcon::Default,
        CursorIcon::Pointer => WinitCursorIcon::Pointer,
        CursorIcon::Crosshair => WinitCursorIcon::Crosshair,
        CursorIcon::Text => WinitCursorIcon::Text,
        CursorIcon::Move => WinitCursorIcon::Move,
        CursorIcon::Wait => WinitCursorIcon::Wait,
        CursorIcon::Help => WinitCursorIcon::Help,
        CursorIcon::NotAllowed => WinitCursorIcon::NotAllowed,
        CursorIcon::ResizeHorizontal => WinitCursorIcon::EwResize,
        CursorIcon::ResizeVertical => WinitCursorIcon::NsResize,
        _ => WinitCursorIcon::Default,
    }
}

fn convert_cursor_grab_mode(mode: CursorGrabMode) -> WinitCursorGrabMode {
    match mode {
        CursorGrabMode::None => WinitCursorGrabMode::None,
        CursorGrabMode::Confined => WinitCursorGrabMode::Confined,
        CursorGrabMode::Locked => WinitCursorGrabMode::Locked,
    }
}
