use engine_platform::{
    ButtonState, CursorEvent, InputEvent, KeyCode, KeyboardEvent, LogicalPosition, Modifiers,
    MouseButton, MouseEvent, MouseWheelDelta, PhysicalPosition, PhysicalSize, PlatformEvent,
    TextInputEvent, TouchEvent, WindowEvent, WindowId,
};
use winit::{
    event::{
        ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, TouchPhase,
        WindowEvent as WinitWindowEvent,
    },
    keyboard::{KeyCode as WinitKeyCode, ModifiersState, PhysicalKey},
};

pub fn convert_window_event(
    engine_window_id: WindowId,
    event: &WinitWindowEvent,
) -> Vec<PlatformEvent> {
    convert_window_event_with_context(engine_window_id, event, Modifiers::default(), 1.0, None)
}

pub(crate) fn convert_window_event_with_context(
    engine_window_id: WindowId,
    event: &WinitWindowEvent,
    modifiers: Modifiers,
    scale_factor: f64,
    current_inner_size: Option<PhysicalSize<u32>>,
) -> Vec<PlatformEvent> {
    let scale_factor = if scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };

    match event {
        WinitWindowEvent::Resized(size) => vec![window_event(
            engine_window_id,
            WindowEvent::Resized {
                size: PhysicalSize {
                    width: size.width,
                    height: size.height,
                },
            },
        )],
        WinitWindowEvent::Moved(position) => vec![window_event(
            engine_window_id,
            WindowEvent::Moved {
                position: PhysicalPosition {
                    x: position.x,
                    y: position.y,
                },
            },
        )],
        WinitWindowEvent::CloseRequested => {
            vec![window_event(engine_window_id, WindowEvent::CloseRequested)]
        }
        WinitWindowEvent::Destroyed => vec![window_event(engine_window_id, WindowEvent::Destroyed)],
        WinitWindowEvent::Focused(focused) => vec![window_event(
            engine_window_id,
            WindowEvent::Focused(*focused),
        )],
        WinitWindowEvent::KeyboardInput { event, .. } => {
            let mut events = vec![input_event(
                Some(engine_window_id),
                InputEvent::Keyboard(KeyboardEvent {
                    key: convert_key_code(&event.physical_key),
                    state: convert_element_state(event.state),
                    repeat: event.repeat,
                    modifiers,
                }),
            )];

            if event.state == ElementState::Pressed {
                if let Some(text) = &event.text {
                    if !text.is_empty() {
                        events.push(input_event(
                            Some(engine_window_id),
                            InputEvent::Text(TextInputEvent {
                                text: text.to_string(),
                            }),
                        ));
                    }
                }
            }

            events
        }
        WinitWindowEvent::CursorMoved { position, .. } => vec![input_event(
            Some(engine_window_id),
            InputEvent::Cursor(CursorEvent::Moved {
                position: LogicalPosition {
                    x: position.x / scale_factor,
                    y: position.y / scale_factor,
                },
            }),
        )],
        WinitWindowEvent::CursorEntered { .. } => vec![input_event(
            Some(engine_window_id),
            InputEvent::Cursor(CursorEvent::Entered),
        )],
        WinitWindowEvent::CursorLeft { .. } => vec![input_event(
            Some(engine_window_id),
            InputEvent::Cursor(CursorEvent::Left),
        )],
        WinitWindowEvent::MouseInput { state, button, .. } => vec![input_event(
            Some(engine_window_id),
            InputEvent::Mouse(MouseEvent::Button {
                button: convert_mouse_button(*button),
                state: convert_element_state(*state),
            }),
        )],
        WinitWindowEvent::MouseWheel { delta, .. } => vec![input_event(
            Some(engine_window_id),
            InputEvent::Mouse(MouseEvent::Wheel {
                delta: convert_mouse_wheel_delta(delta),
            }),
        )],
        WinitWindowEvent::Touch(touch) => convert_touch(touch, scale_factor)
            .into_iter()
            .map(|event| input_event(Some(engine_window_id), InputEvent::Touch(event)))
            .collect(),
        WinitWindowEvent::ScaleFactorChanged { scale_factor, .. } => vec![window_event(
            engine_window_id,
            WindowEvent::ScaleFactorChanged {
                scale_factor: *scale_factor,
                new_inner_size: current_inner_size.unwrap_or(PhysicalSize {
                    width: 0,
                    height: 0,
                }),
            },
        )],
        WinitWindowEvent::RedrawRequested => {
            vec![PlatformEvent::RedrawRequested {
                window: engine_window_id,
            }]
        }
        WinitWindowEvent::ModifiersChanged(_) => Vec::new(),
        _ => Vec::new(),
    }
}

pub fn convert_key_code(physical_key: &PhysicalKey) -> KeyCode {
    let PhysicalKey::Code(code) = physical_key else {
        return KeyCode::Unknown;
    };

    match code {
        WinitKeyCode::Escape => KeyCode::Escape,
        WinitKeyCode::Enter => KeyCode::Enter,
        WinitKeyCode::Space => KeyCode::Space,
        WinitKeyCode::Tab => KeyCode::Tab,
        WinitKeyCode::Backspace => KeyCode::Backspace,
        WinitKeyCode::ShiftLeft => KeyCode::ShiftLeft,
        WinitKeyCode::ShiftRight => KeyCode::ShiftRight,
        WinitKeyCode::ControlLeft => KeyCode::ControlLeft,
        WinitKeyCode::ControlRight => KeyCode::ControlRight,
        WinitKeyCode::AltLeft => KeyCode::AltLeft,
        WinitKeyCode::AltRight => KeyCode::AltRight,
        WinitKeyCode::ArrowUp => KeyCode::ArrowUp,
        WinitKeyCode::ArrowDown => KeyCode::ArrowDown,
        WinitKeyCode::ArrowLeft => KeyCode::ArrowLeft,
        WinitKeyCode::ArrowRight => KeyCode::ArrowRight,
        WinitKeyCode::Digit0 => KeyCode::Digit0,
        WinitKeyCode::Digit1 => KeyCode::Digit1,
        WinitKeyCode::Digit2 => KeyCode::Digit2,
        WinitKeyCode::Digit3 => KeyCode::Digit3,
        WinitKeyCode::Digit4 => KeyCode::Digit4,
        WinitKeyCode::Digit5 => KeyCode::Digit5,
        WinitKeyCode::Digit6 => KeyCode::Digit6,
        WinitKeyCode::Digit7 => KeyCode::Digit7,
        WinitKeyCode::Digit8 => KeyCode::Digit8,
        WinitKeyCode::Digit9 => KeyCode::Digit9,
        WinitKeyCode::KeyA => KeyCode::KeyA,
        WinitKeyCode::KeyB => KeyCode::KeyB,
        WinitKeyCode::KeyC => KeyCode::KeyC,
        WinitKeyCode::KeyD => KeyCode::KeyD,
        WinitKeyCode::KeyE => KeyCode::KeyE,
        WinitKeyCode::KeyF => KeyCode::KeyF,
        WinitKeyCode::KeyG => KeyCode::KeyG,
        WinitKeyCode::KeyH => KeyCode::KeyH,
        WinitKeyCode::KeyI => KeyCode::KeyI,
        WinitKeyCode::KeyJ => KeyCode::KeyJ,
        WinitKeyCode::KeyK => KeyCode::KeyK,
        WinitKeyCode::KeyL => KeyCode::KeyL,
        WinitKeyCode::KeyM => KeyCode::KeyM,
        WinitKeyCode::KeyN => KeyCode::KeyN,
        WinitKeyCode::KeyO => KeyCode::KeyO,
        WinitKeyCode::KeyP => KeyCode::KeyP,
        WinitKeyCode::KeyQ => KeyCode::KeyQ,
        WinitKeyCode::KeyR => KeyCode::KeyR,
        WinitKeyCode::KeyS => KeyCode::KeyS,
        WinitKeyCode::KeyT => KeyCode::KeyT,
        WinitKeyCode::KeyU => KeyCode::KeyU,
        WinitKeyCode::KeyV => KeyCode::KeyV,
        WinitKeyCode::KeyW => KeyCode::KeyW,
        WinitKeyCode::KeyX => KeyCode::KeyX,
        WinitKeyCode::KeyY => KeyCode::KeyY,
        WinitKeyCode::KeyZ => KeyCode::KeyZ,
        WinitKeyCode::F1 => KeyCode::F(1),
        WinitKeyCode::F2 => KeyCode::F(2),
        WinitKeyCode::F3 => KeyCode::F(3),
        WinitKeyCode::F4 => KeyCode::F(4),
        WinitKeyCode::F5 => KeyCode::F(5),
        WinitKeyCode::F6 => KeyCode::F(6),
        WinitKeyCode::F7 => KeyCode::F(7),
        WinitKeyCode::F8 => KeyCode::F(8),
        WinitKeyCode::F9 => KeyCode::F(9),
        WinitKeyCode::F10 => KeyCode::F(10),
        WinitKeyCode::F11 => KeyCode::F(11),
        WinitKeyCode::F12 => KeyCode::F(12),
        WinitKeyCode::F13 => KeyCode::F(13),
        WinitKeyCode::F14 => KeyCode::F(14),
        WinitKeyCode::F15 => KeyCode::F(15),
        WinitKeyCode::F16 => KeyCode::F(16),
        WinitKeyCode::F17 => KeyCode::F(17),
        WinitKeyCode::F18 => KeyCode::F(18),
        WinitKeyCode::F19 => KeyCode::F(19),
        WinitKeyCode::F20 => KeyCode::F(20),
        WinitKeyCode::F21 => KeyCode::F(21),
        WinitKeyCode::F22 => KeyCode::F(22),
        WinitKeyCode::F23 => KeyCode::F(23),
        WinitKeyCode::F24 => KeyCode::F(24),
        _ => KeyCode::Unknown,
    }
}

pub fn convert_element_state(state: ElementState) -> ButtonState {
    match state {
        ElementState::Pressed => ButtonState::Pressed,
        ElementState::Released => ButtonState::Released,
    }
}

pub fn convert_mouse_button(button: WinitMouseButton) -> MouseButton {
    match button {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Back => MouseButton::Back,
        WinitMouseButton::Forward => MouseButton::Forward,
        WinitMouseButton::Other(button) => MouseButton::Other(button),
    }
}

pub fn convert_modifiers(modifiers: ModifiersState) -> Modifiers {
    Modifiers {
        shift: modifiers.shift_key(),
        ctrl: modifiers.control_key(),
        alt: modifiers.alt_key(),
        logo: modifiers.super_key(),
    }
}

fn convert_mouse_wheel_delta(delta: &MouseScrollDelta) -> MouseWheelDelta {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => MouseWheelDelta::Lines { x: *x, y: *y },
        MouseScrollDelta::PixelDelta(position) => MouseWheelDelta::Pixels {
            x: position.x as f32,
            y: position.y as f32,
        },
    }
}

fn convert_touch(touch: &winit::event::Touch, scale_factor: f64) -> Option<TouchEvent> {
    let position = LogicalPosition {
        x: touch.location.x / scale_factor,
        y: touch.location.y / scale_factor,
    };

    match touch.phase {
        TouchPhase::Started => Some(TouchEvent::Started {
            id: touch.id,
            position,
        }),
        TouchPhase::Moved => Some(TouchEvent::Moved {
            id: touch.id,
            position,
        }),
        TouchPhase::Ended => Some(TouchEvent::Ended {
            id: touch.id,
            position,
        }),
        TouchPhase::Cancelled => Some(TouchEvent::Cancelled { id: touch.id }),
    }
}

fn window_event(id: WindowId, event: WindowEvent) -> PlatformEvent {
    PlatformEvent::Window { id, event }
}

fn input_event(window: Option<WindowId>, event: InputEvent) -> PlatformEvent {
    PlatformEvent::Input { window, event }
}
