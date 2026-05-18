pub mod error;
pub mod surface;

pub use error::{GraphicsError, GraphicsResult};
pub use surface::{Color, PresentMode, RenderSurface, SurfaceSize};
