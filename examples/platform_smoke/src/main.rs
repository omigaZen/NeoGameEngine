use engine_platform::{
    ButtonState, InputEvent, KeyCode, Platform, PlatformApp, PlatformContext, PlatformEvent,
    PlatformResult, RunMode, WindowDesc, WindowEvent, WindowId,
};
use platform_winit::WinitPlatform;

struct SmokeApp {
    window: Option<WindowId>,
    frame_count: u64,
}

impl SmokeApp {
    fn new() -> Self {
        Self {
            window: None,
            frame_count: 0,
        }
    }
}

impl PlatformApp for SmokeApp {
    fn on_resumed(&mut self, ctx: &mut dyn PlatformContext) -> PlatformResult<()> {
        if self.window.is_none() {
            let desc = WindowDesc {
                title: "Platform Smoke Test".to_owned(),
                ..WindowDesc::default()
            };
            let window = ctx.create_window(desc)?;
            self.window = Some(window);
        }

        ctx.set_run_mode(RunMode::Poll);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut dyn PlatformContext,
        event: PlatformEvent,
    ) -> PlatformResult<()> {
        match event {
            PlatformEvent::Window { id, event } => {
                println!("window {id:?}: {event:?}");

                match event {
                    WindowEvent::CloseRequested => {
                        ctx.destroy_window(id)?;
                        self.window = None;
                        ctx.exit();
                    }
                    WindowEvent::Resized { size } => {
                        println!("resized: {}x{}", size.width, size.height);
                    }
                    _ => {}
                }
            }
            PlatformEvent::Input { window, event } => {
                println!("input {window:?}: {event:?}");

                if let InputEvent::Keyboard(keyboard) = event {
                    if keyboard.key == KeyCode::Escape && keyboard.state == ButtonState::Pressed {
                        ctx.exit();
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn on_update(
        &mut self,
        ctx: &mut dyn PlatformContext,
        _dt: std::time::Duration,
    ) -> PlatformResult<()> {
        if let Some(window) = self.window {
            ctx.request_redraw(window);
        }
        Ok(())
    }

    fn on_redraw(
        &mut self,
        _ctx: &mut dyn PlatformContext,
        window_id: WindowId,
    ) -> PlatformResult<()> {
        if Some(window_id) == self.window {
            self.frame_count += 1;
            if self.frame_count.is_multiple_of(60) {
                println!("redraw {}", self.frame_count);
            }
        }
        Ok(())
    }
}

fn main() -> PlatformResult<()> {
    WinitPlatform::new().run(Box::new(SmokeApp::new()))
}
