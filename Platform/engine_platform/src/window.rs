use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::PlatformResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalSize<T> {
    pub width: T,
    pub height: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalSize<T> {
    pub width: T,
    pub height: T,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalPosition<T> {
    pub x: T,
    pub y: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalPosition<T> {
    pub x: T,
    pub y: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullscreenMode {
    Windowed,
    Borderless,
}

#[derive(Debug, Clone)]
pub struct WindowDesc {
    pub title: String,
    pub size: LogicalSize<f64>,
    pub min_size: Option<LogicalSize<f64>>,
    pub max_size: Option<LogicalSize<f64>>,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub visible: bool,
    pub fullscreen: FullscreenMode,
}

impl Default for WindowDesc {
    fn default() -> Self {
        Self {
            title: "Game".to_owned(),
            size: LogicalSize {
                width: 1280.0,
                height: 720.0,
            },
            min_size: None,
            max_size: None,
            resizable: true,
            decorations: true,
            transparent: false,
            visible: true,
            fullscreen: FullscreenMode::Windowed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorGrabMode {
    None,
    Confined,
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CursorIcon {
    Default,
    Pointer,
    Crosshair,
    Text,
    Move,
    Wait,
    Help,
    NotAllowed,
    ResizeHorizontal,
    ResizeVertical,
}

pub trait PlatformWindow: HasWindowHandle + HasDisplayHandle {
    fn id(&self) -> WindowId;

    fn title(&self) -> String;
    fn set_title(&self, title: &str);

    fn inner_size(&self) -> PhysicalSize<u32>;
    fn outer_size(&self) -> PhysicalSize<u32>;
    fn scale_factor(&self) -> f64;

    fn is_focused(&self) -> bool;
    fn is_visible(&self) -> bool;

    fn set_visible(&self, visible: bool);
    fn set_resizable(&self, resizable: bool);

    fn set_cursor_visible(&self, visible: bool);
    fn set_cursor_icon(&self, icon: CursorIcon);
    fn set_cursor_grab(&self, mode: CursorGrabMode) -> PlatformResult<()>;

    fn set_cursor_position(&self, position: LogicalPosition<f64>) -> PlatformResult<()>;

    fn request_redraw(&self);
}
