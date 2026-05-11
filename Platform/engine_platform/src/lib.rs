pub mod error;
pub mod fs;
pub mod input;
pub mod window;

pub use error::{PlatformError, PlatformResult};
pub use fs::FileSystem;
pub use input::*;
pub use window::*;

pub mod event;
pub mod time;

pub use event::{PlatformEvent, WindowEvent};
pub use time::RunMode;

pub trait Platform {
    fn run(self, app: Box<dyn PlatformApp>) -> PlatformResult<()>;
}

pub trait PlatformApp {
    fn on_resumed(&mut self, _ctx: &mut dyn PlatformContext) -> PlatformResult<()> {
        Ok(())
    }

    fn on_suspended(&mut self, _ctx: &mut dyn PlatformContext) {}

    fn on_event(
        &mut self,
        _ctx: &mut dyn PlatformContext,
        _event: PlatformEvent,
    ) -> PlatformResult<()> {
        Ok(())
    }

    fn on_update(
        &mut self,
        _ctx: &mut dyn PlatformContext,
        _dt: std::time::Duration,
    ) -> PlatformResult<()> {
        Ok(())
    }

    fn on_redraw(
        &mut self,
        _ctx: &mut dyn PlatformContext,
        _window_id: WindowId,
    ) -> PlatformResult<()> {
        Ok(())
    }

    fn on_shutdown(&mut self, _ctx: &mut dyn PlatformContext) {}
}

pub trait PlatformContext {
    fn create_window(&mut self, desc: WindowDesc) -> PlatformResult<WindowId>;

    fn destroy_window(&mut self, id: WindowId) -> PlatformResult<()>;

    fn window(&self, id: WindowId) -> Option<&dyn PlatformWindow>;

    fn primary_window(&self) -> Option<WindowId>;

    fn request_redraw(&mut self, id: WindowId);

    fn request_redraw_all(&mut self);

    fn set_run_mode(&mut self, mode: RunMode);

    fn exit(&mut self);

    fn now(&self) -> std::time::Instant;

    fn file_system(&self) -> &dyn FileSystem;
}
