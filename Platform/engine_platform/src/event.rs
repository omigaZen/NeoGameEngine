use crate::{InputEvent, PhysicalPosition, PhysicalSize, WindowId};

#[derive(Debug, Clone)]
pub enum PlatformEvent {
    AppResumed,
    AppSuspended,

    Window {
        id: WindowId,
        event: WindowEvent,
    },

    Input {
        window: Option<WindowId>,
        event: InputEvent,
    },

    RedrawRequested {
        window: WindowId,
    },

    AboutToWait,
}

#[derive(Debug, Clone)]
pub enum WindowEvent {
    Created,
    CloseRequested,
    Destroyed,

    Focused(bool),

    Resized {
        size: PhysicalSize<u32>,
    },

    ScaleFactorChanged {
        scale_factor: f64,
        new_inner_size: PhysicalSize<u32>,
    },

    Moved {
        position: PhysicalPosition<i32>,
    },
}
