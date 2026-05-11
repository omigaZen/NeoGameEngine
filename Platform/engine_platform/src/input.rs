#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KeyCode {
    Escape,
    Enter,
    Space,
    Tab,
    Backspace,

    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,

    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,

    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,

    F(u8),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseWheelDelta {
    Lines { x: f32, y: f32 },
    Pixels { x: f32, y: f32 },
}

#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    pub key: KeyCode,
    pub state: ButtonState,
    pub repeat: bool,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone)]
pub struct TextInputEvent {
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum MouseEvent {
    Button {
        button: MouseButton,
        state: ButtonState,
    },
    Wheel {
        delta: MouseWheelDelta,
    },
}

#[derive(Debug, Clone)]
pub enum CursorEvent {
    Moved {
        position: crate::LogicalPosition<f64>,
    },
    Entered,
    Left,
}

#[derive(Debug, Clone)]
pub enum TouchEvent {
    Started {
        id: u64,
        position: crate::LogicalPosition<f64>,
    },
    Moved {
        id: u64,
        position: crate::LogicalPosition<f64>,
    },
    Ended {
        id: u64,
        position: crate::LogicalPosition<f64>,
    },
    Cancelled {
        id: u64,
    },
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    Keyboard(KeyboardEvent),
    Text(TextInputEvent),
    Mouse(MouseEvent),
    Cursor(CursorEvent),
    Touch(TouchEvent),
}
