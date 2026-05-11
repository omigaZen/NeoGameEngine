use std::time::Instant;

use engine_platform::{
    Platform, PlatformApp, PlatformError, PlatformEvent, PlatformResult, RunMode,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent as WinitWindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId as WinitRawWindowId,
};

use crate::{
    context::{WinitFrameContext, WinitState},
    convert::{convert_modifiers, convert_window_event_with_context},
};

pub struct WinitPlatform;

impl WinitPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WinitPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for WinitPlatform {
    fn run(self, app: Box<dyn PlatformApp>) -> PlatformResult<()> {
        let event_loop =
            EventLoop::new().map_err(|err| PlatformError::BackendError(err.to_string()))?;
        let mut handler = Handler::new(app);
        let run_result = event_loop.run_app(&mut handler);

        if let Some(error) = handler.error {
            Err(error)
        } else {
            run_result.map_err(|err| PlatformError::BackendError(err.to_string()))
        }
    }
}

struct Handler {
    app: Box<dyn PlatformApp>,
    state: WinitState,
    last_frame_time: Instant,
    error: Option<PlatformError>,
}

impl Handler {
    fn new(app: Box<dyn PlatformApp>) -> Self {
        Self {
            app,
            state: WinitState::new(),
            last_frame_time: Instant::now(),
            error: None,
        }
    }

    fn record_error(&mut self, event_loop: &ActiveEventLoop, error: PlatformError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
        event_loop.exit();
    }

    fn finish_callback(&mut self, event_loop: &ActiveEventLoop, result: PlatformResult<()>) {
        if let Err(error) = result {
            self.record_error(event_loop, error);
            return;
        }

        self.dispatch_pending_events(event_loop);

        if self.state.should_exit() {
            event_loop.exit();
        }
    }

    fn dispatch_pending_events(&mut self, event_loop: &ActiveEventLoop) {
        while self.error.is_none() {
            let events = self.state.take_pending_events();
            if events.is_empty() {
                break;
            }

            for event in events {
                if let Err(error) = self.dispatch_platform_event(event_loop, event) {
                    self.record_error(event_loop, error);
                    return;
                }

                if self.state.should_exit() {
                    event_loop.exit();
                    return;
                }
            }
        }
    }

    fn dispatch_platform_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: PlatformEvent,
    ) -> PlatformResult<()> {
        match event {
            PlatformEvent::RedrawRequested { window } => {
                let mut ctx = WinitFrameContext::new(event_loop, &mut self.state);
                self.app.on_redraw(&mut ctx, window)
            }
            event => {
                let mut ctx = WinitFrameContext::new(event_loop, &mut self.state);
                self.app.on_event(&mut ctx, event)
            }
        }
    }

    fn dispatch_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WinitRawWindowId,
        event: WinitWindowEvent,
    ) {
        if self.error.is_some() {
            return;
        }

        let Some(engine_window_id) = self.state.window_id_for(window_id) else {
            return;
        };

        if let WinitWindowEvent::ModifiersChanged(modifiers) = &event {
            self.state
                .set_current_modifiers(convert_modifiers(modifiers.state()));
        }

        let scale_factor = self.state.window_scale_factor(engine_window_id);
        let inner_size = self.state.window_inner_size(engine_window_id);
        let events = convert_window_event_with_context(
            engine_window_id,
            &event,
            self.state.current_modifiers(),
            scale_factor,
            inner_size,
        );

        for event in events {
            if let Err(error) = self.dispatch_platform_event(event_loop, event) {
                self.record_error(event_loop, error);
                return;
            }

            self.dispatch_pending_events(event_loop);

            if self.state.should_exit() {
                event_loop.exit();
                return;
            }
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = {
            let mut ctx = WinitFrameContext::new(event_loop, &mut self.state);
            self.app.on_resumed(&mut ctx)
        };
        self.finish_callback(event_loop, result);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        {
            let mut ctx = WinitFrameContext::new(event_loop, &mut self.state);
            self.app.on_suspended(&mut ctx);
        }
        self.dispatch_pending_events(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WinitRawWindowId,
        event: WinitWindowEvent,
    ) {
        self.dispatch_window_event(event_loop, window_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.error.is_some() {
            event_loop.exit();
            return;
        }

        if self.state.should_exit() {
            event_loop.exit();
            return;
        }

        let now = Instant::now();
        let dt = now.saturating_duration_since(self.last_frame_time);
        self.last_frame_time = now;

        let about_to_wait_result =
            self.dispatch_platform_event(event_loop, PlatformEvent::AboutToWait);
        if let Err(error) = about_to_wait_result {
            self.record_error(event_loop, error);
            return;
        }

        let update_result = {
            let mut ctx = WinitFrameContext::new(event_loop, &mut self.state);
            self.app.on_update(&mut ctx, dt)
        };
        self.finish_callback(event_loop, update_result);

        if self.error.is_some() || self.state.should_exit() {
            event_loop.exit();
            return;
        }

        match self.state.run_mode() {
            RunMode::Poll => event_loop.set_control_flow(ControlFlow::Poll),
            RunMode::Wait => event_loop.set_control_flow(ControlFlow::Wait),
            RunMode::WaitUntil(instant) => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(instant))
            }
        }

        if matches!(self.state.run_mode(), RunMode::Poll) {
            self.state.request_redraw_all();
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let mut ctx = WinitFrameContext::new(event_loop, &mut self.state);
        self.app.on_shutdown(&mut ctx);
    }
}
